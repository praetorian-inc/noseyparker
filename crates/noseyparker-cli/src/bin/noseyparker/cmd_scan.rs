use anyhow::{bail, Context, Result};
use bstr::ByteSlice;
use indicatif::{HumanBytes, HumanCount, HumanDuration};
use rayon::prelude::*;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{debug, debug_span, error, info, warn};

use crate::args;

use noseyparker::blob::{Blob, BlobId};
use noseyparker::blob_id_set::BlobIdSet;
use noseyparker::blob_metadata::BlobMetadata;
use noseyparker::datastore::Datastore;
use noseyparker::defaults::DEFAULT_IGNORE_RULES;
use noseyparker::git_binary::{CloneMode, Git};
use noseyparker::git_url::GitUrl;
use noseyparker::github;
use noseyparker::input_enumerator::{open_git_repo, FileResult, FilesystemEnumerator};
use noseyparker::location;
use noseyparker::match_type::Match;
use noseyparker::matcher::Matcher;
use noseyparker::matcher_stats::MatcherStats;
use noseyparker::progress::Progress;
use noseyparker::provenance::{CommitKind, Provenance};
use noseyparker::rules::Rules;
use noseyparker::rules_database::RulesDatabase;
use noseyparker::{content_guesser, content_guesser::Guesser};

/// This command scans multiple filesystem inputs for secrets.
/// The implementation enumerates content in parallel, scans the enumerated content in parallel,
/// and records found matches to a SQLite database from a single dedicated thread.
pub fn run(global_args: &args::GlobalArgs, args: &args::ScanArgs) -> Result<()> {
    let _span = debug_span!("scan").entered();

    debug!("Args: {args:#?}");

    let progress_enabled = global_args.use_progress();

    // ---------------------------------------------------------------------------------------------
    // Configure the Rayon global thread pool
    // ---------------------------------------------------------------------------------------------
    debug!("Using {} parallel jobs", args.num_jobs);
    rayon::ThreadPoolBuilder::new()
        .num_threads(args.num_jobs)
        .thread_name(|idx| format!("Scanner {idx}"))
        .build_global()
        .with_context(|| format!("Failed to configure Rayon with {} threads", args.num_jobs))?;

    // ---------------------------------------------------------------------------------------------
    // Open datastore
    // ---------------------------------------------------------------------------------------------
    let mut datastore = Datastore::create_or_open(&args.datastore)?;

    // ---------------------------------------------------------------------------------------------
    // Load rules
    // ---------------------------------------------------------------------------------------------
    let rules_db = {
        let mut rules = Rules::from_default_rules().context("Failed to load default rules")?;
        if !args.rules.is_empty() {
            let custom_rules =
                Rules::from_paths(&args.rules).context("Failed to load specified rules files")?;
            rules.extend(custom_rules);
        }
        RulesDatabase::from_rules(rules).context("Failed to compile rules")?
    };

    // ---------------------------------------------------------------------------------------------
    // Enumerate any mentioned GitHub repositories; gather list of all repos to clone or update
    // ---------------------------------------------------------------------------------------------
    let repo_urls = {
        let repo_specifiers = github::RepoSpecifiers {
            user: args.input_specifier_args.github_user.clone(),
            organization: args.input_specifier_args.github_organization.clone(),
        };
        let mut repo_urls = args.input_specifier_args.git_url.clone();
        if !repo_specifiers.is_empty() {
            let mut progress = Progress::new_countup_spinner(
                "Enumerating GitHub repositories...",
                progress_enabled,
            );
            let mut num_found: u64 = 0;
            let api_url = args.input_specifier_args.github_api_url.clone();
            for repo_string in
                github::enumerate_repo_urls(&repo_specifiers, api_url, Some(&mut progress))
                    .context("Failed to enumerate GitHub repositories")?
            {
                match GitUrl::from_str(&repo_string) {
                    Ok(repo_url) => repo_urls.push(repo_url),
                    Err(e) => {
                        progress.suspend(|| {
                            error!("Failed to parse repo URL from {repo_string}: {e}");
                        });
                        continue;
                    }
                }
                num_found += 1;
            }

            progress.finish_with_message(format!(
                "Found {} repositories from GitHub",
                HumanCount(num_found)
            ));
        }
        repo_urls.sort();
        repo_urls.dedup();

        repo_urls
    };

    // ---------------------------------------------------------------------------------------------
    // Clone or update all mentioned Git URLs
    // ---------------------------------------------------------------------------------------------
    if !repo_urls.is_empty() {
        info!("{} Git URLs to fetch", repo_urls.len());
    }
    for repo_url in &repo_urls {
        debug!("Need to fetch {repo_url}")
    }

    let mut input_roots = args.input_specifier_args.path_inputs.clone();

    if !repo_urls.is_empty() {
        let clone_mode = match args.input_specifier_args.git_clone_mode {
            args::GitCloneMode::Mirror => CloneMode::Mirror,
            args::GitCloneMode::Bare => CloneMode::Bare,
        };
        let git = Git::new();

        let mut progress =
            Progress::new_bar(repo_urls.len() as u64, "Fetching Git repos", progress_enabled);

        for repo_url in repo_urls {
            let output_dir = match datastore.clone_destination(&repo_url) {
                Err(e) => {
                    progress.suspend(|| {
                        error!("Failed to determine output directory for {repo_url}: {e}");
                        warn!("Skipping scan of {repo_url}");
                    });
                    progress.inc(1);
                    continue;
                }
                Ok(output_dir) => output_dir,
            };

            // First, try to update an existing clone, and if that fails, do a fresh clone
            if output_dir.is_dir() {
                progress.suspend(|| info!("Updating clone of {repo_url}..."));

                match git.update_clone(&repo_url, &output_dir) {
                    Ok(()) => {
                        input_roots.push(output_dir);
                        progress.inc(1);
                        continue;
                    }
                    Err(e) => {
                        progress.suspend(|| {
                            warn!(
                                "Failed to update clone of {repo_url} at {}: {e}",
                                output_dir.display()
                            )
                        });
                        if let Err(e) = std::fs::remove_dir_all(&output_dir) {
                            progress.suspend(|| {
                                error!(
                                    "Failed to remove clone directory at {}: {e}",
                                    output_dir.display()
                                )
                            });
                        }
                    }
                }
            }

            progress.suspend(|| info!("Cloning {repo_url}..."));
            if let Err(e) = git.create_fresh_clone(&repo_url, &output_dir, clone_mode) {
                progress.suspend(|| {
                    error!("Failed to clone {repo_url} to {}: {e}", output_dir.display());
                    warn!("Skipping scan of {repo_url}");
                });
                progress.inc(1);
                continue;
            }
            input_roots.push(output_dir);
            progress.inc(1);
        }

        progress.finish();
    }

    if input_roots.is_empty() {
        bail!("No inputs to scan");
    }

    // ---------------------------------------------------------------------------------------------
    // Enumerate initial filesystem inputs
    // ---------------------------------------------------------------------------------------------
    let inputs = {
        let mut progress = Progress::new_bytes_spinner("Enumerating inputs...", progress_enabled);

        let input_enumerator = || -> Result<FilesystemEnumerator> {
            let mut ie = FilesystemEnumerator::new(&input_roots)?;
            ie.threads(args.num_jobs);
            ie.max_filesize(args.content_filtering_args.max_file_size_bytes());

            // Load default ignore file. Note that we have to write it to a file first,
            // because the API for the `ignore` crate doesn't expose something that takes a
            // string.
            let ignore_path = datastore.scratch_dir().join("default_ignore_rules.conf");
            std::fs::write(&ignore_path, DEFAULT_IGNORE_RULES).with_context(|| {
                format!("Failed to write default ignore rules to {}", ignore_path.display())
            })?;

            ie.add_ignore(&ignore_path).with_context(|| {
                format!("Failed to load ignore rules from {}", ignore_path.display())
            })?;

            // Load any specified ignore files
            for ignore_path in args.content_filtering_args.ignore.iter() {
                debug!("Using ignore rules from {}", ignore_path.display());
                ie.add_ignore(ignore_path).with_context(|| {
                    format!("Failed to load ignore rules from {}", ignore_path.display())
                })?;
            }

            // Make sure the datastore itself is not scanned
            let datastore_path = std::fs::canonicalize(datastore.root_dir())?;
            ie.filter_entry(move |entry| {
                let path = match std::fs::canonicalize(entry.path()) {
                    Err(e) => {
                        warn!("Failed to canonicalize path {}: {}", entry.path().display(), e);
                        return true;
                    }
                    Ok(p) => p,
                };
                path != datastore_path
            });

            // Determine whether to collect git metadata or not
            let collect_git_metadata = match args.metadata_args.git_blob_provenance {
                args::GitBlobProvenanceMode::FirstSeen => true,
                args::GitBlobProvenanceMode::Minimal => false,
            };
            ie.collect_git_metadata(collect_git_metadata);

            Ok(ie)
        }()
        .context("Failed to initialize filesystem enumerator")?;

        let inputs = input_enumerator
            .run(&progress)
            .context("Failed to enumerate filesystem inputs")?;
        let total_bytes_found: u64 = {
            let blob_bytes: u64 = inputs.git_repos.iter().map(|r| r.total_blob_bytes()).sum();
            let file_bytes: u64 = inputs.files.iter().map(|e| e.num_bytes).sum();
            blob_bytes + file_bytes
        };
        progress.finish_with_message(format!(
            "Found {} from {} plain files and {} blobs from {} Git repos",
            HumanBytes(total_bytes_found),
            HumanCount(inputs.files.len() as u64),
            HumanCount(inputs.git_repos.iter().map(|r| r.num_blobs()).sum()),
            HumanCount(inputs.git_repos.len() as u64),
        ));
        inputs
    };

    // ---------------------------------------------------------------------------------------------
    // Define some matcher helper code and shared state
    // ---------------------------------------------------------------------------------------------
    let scan_start = Instant::now();
    let total_blob_bytes = inputs.total_blob_bytes();

    let num_matchers_counter = Mutex::new(0u64); // how many matchers have been initialized?
    let matcher_stats = Mutex::new(MatcherStats::default());
    let seen_blobs = BlobIdSet::new();

    let make_matcher = || -> Result<(Matcher, Guesser)> {
        *num_matchers_counter.lock().unwrap() += 1;
        let matcher = Matcher::new(&rules_db, &seen_blobs, Some(&matcher_stats))?;
        let guesser = content_guesser::Guesser::new()?;
        Ok((matcher, guesser))
    };

    // FIXME: have this print out aggregate rate at finish
    let mut progress =
        Progress::new_bytes_bar(total_blob_bytes, "Scanning content", progress_enabled);

    // Create a channel pair for matcher threads to get their results to the datastore recorder.
    // let channel_size = std::cmp::max(args.num_jobs * 32, 1024);
    type DatastoreMessage = (BlobMetadata, Vec<Match>);
    // let (send_ds, recv_ds) = crossbeam_channel::bounded::<DatastoreMessage>(channel_size);
    let (send_ds, recv_ds) = crossbeam_channel::unbounded::<DatastoreMessage>();

    // We create a separate thread for writing matches to the datastore.
    // The datastore uses SQLite, which does best with a single writer.
    let datastore_writer = {
        std::thread::Builder::new()
            .name("Datastore Writer".to_string())
            .spawn(move || {
                let mut num_added: u64 = 0;

                // keep reading until all the senders hang up; panic if recording matches fails
                //
                // accumulate messages in batches to avoid an excessive number of tiny datastore
                // transactions (which kills performance); try to commit at least every second

                let mut last_tx_time = std::time::Instant::now();

                // FIXME: make the following CLI parameters
                const BUF_SIZE: usize = 16384;
                const COMMIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(1000);

                #[derive(Default)]
                struct BatchMatches {
                    batch_matches: Vec<Vec<Match>>,
                    count: usize,
                }

                impl BatchMatches {
                    fn with_capacity(capacity: usize) -> Self {
                        Self {
                            count: 0,
                            batch_matches: Vec::with_capacity(capacity),
                        }
                    }

                    fn len(&self) -> u64 {
                        self.count.try_into().unwrap()
                    }

                    fn push(&mut self, matches: Vec<Match>) {
                        self.count += matches.len();
                        self.batch_matches.push(matches);
                    }

                    fn take(&mut self) -> impl Iterator<Item = Match> {
                        self.count = 0;
                        let ms = std::mem::take(&mut self.batch_matches);
                        ms.into_iter().flatten()
                    }

                    fn is_empty(&self) -> bool {
                        self.count == 0
                    }
                }

                let mut batch_matches = BatchMatches::with_capacity(BUF_SIZE);
                let mut batch_metadata: Vec<BlobMetadata> = Vec::with_capacity(BUF_SIZE);

                for (metadata, matches) in recv_ds.iter() {
                    batch_matches.push(matches);
                    batch_metadata.push(metadata);

                    if batch_matches.len() as usize >= BUF_SIZE
                        || batch_metadata.len() >= BUF_SIZE
                        || last_tx_time.elapsed() >= COMMIT_INTERVAL
                    {
                        // let t1 = std::time::Instant::now();
                        num_added += datastore
                            .record_metadata_and_matches(std::mem::take(&mut batch_metadata), batch_matches.take())
                            .expect("should be able to record blob metadata and matches to the datastore");
                        // debug!("*** commit metadata: {:.3}s {} {}", t1.elapsed().as_secs_f64(), batch_metadata.len(), recv_ds.len());

                        last_tx_time = std::time::Instant::now();
                    }
                }

                if !batch_metadata.is_empty() || !batch_matches.is_empty() {
                    // let t1 = std::time::Instant::now();
                   num_added +=  datastore
                        .record_metadata_and_matches(std::mem::take(&mut batch_metadata), batch_matches.take())
                        .expect("should be able to record blob metadata and matches to the datastore");
                    // debug!("*** commit metadata: {:.3}s {} {}", t1.elapsed().as_secs_f64(), batch_metadata.len(), recv_ds.len());
                }

                let num_matches = datastore.get_num_matches()
                    .expect("should be able to count number of matches in datastore");

                datastore
                    .analyze()
                    .expect("should be able to analyze the datastore");
                (datastore, num_matches, num_added)
            })
            .expect("should be able to start datastore writer thread")
    };

    // ---------------------------------------------------------------------------------------------
    // Scan plain files
    // ---------------------------------------------------------------------------------------------
    inputs.files.par_iter().try_for_each_init(
        || {
            let matcher = make_matcher().expect("should be able to create a matcher");

            (matcher, progress.clone())
        },
        |(matcher, progress), file_result: &FileResult| -> Result<()> {
            let fname = &file_result.path;
            let blob = match Blob::from_file(fname) {
                Err(e) => {
                    error!("Failed to load blob from {}: {}", fname.display(), e);
                    return Ok(());
                }
                Ok(v) => v,
            };
            progress.inc(file_result.num_bytes);

            run_matcher(
                matcher,
                Provenance::from_file(fname),
                blob,
                &send_ds,
                args.snippet_length,
                args.metadata_args.blob_metadata,
                &progress,
            )?;

            Ok(())
        },
    )?;

    // ---------------------------------------------------------------------------------------------
    // Scan Git repo inputs
    // ---------------------------------------------------------------------------------------------
    inputs
        .git_repos
        .into_par_iter()
        .try_for_each(|git_repo_result| -> Result<()> {
            let repository = match open_git_repo(&git_repo_result.path) {
                Ok(Some(repository)) => repository.into_sync(),
                Ok(None) => {
                    error!(
                        "Failed to re-open previously-found repository at {}",
                        git_repo_result.path.display()
                    );
                    return Ok(());
                }
                Err(err) => {
                    error!(
                        "Failed to re-open previously-found repository at {}: {err}",
                        git_repo_result.path.display()
                    );
                    return Ok(());
                }
            };

            git_repo_result.blobs.into_par_iter().try_for_each_init(
                || {
                    let repo = repository.to_thread_local();
                    let matcher = make_matcher().expect("should be able to create a matcher");
                    (repo, matcher, progress.clone())
                },
                |(repo, matcher, progress), md| -> Result<()> {
                    let size = md.num_bytes;
                    let blob_id = md.blob_oid;
                    progress.inc(size);
                    let repo_path = &git_repo_result.path;
                    // debug!("Scanning {} size {} from {:?}", oid, size, path);

                    let blob = match repo.find_object(blob_id) {
                        Err(e) => {
                            error!(
                                "Failed to read blob {blob_id} from Git repository at {}: {e}",
                                repo_path.display(),
                            );
                            return Ok(());
                        }
                        Ok(mut blob) => {
                            let data = std::mem::take(&mut blob.data); // avoid a copy
                            Blob::new(BlobId::from(&blob_id), data)
                        }
                    };

                    // FIXME: use _all_ provenance values
                    let provenance = if md.first_seen.is_empty() {
                        Provenance::from_git_repo(repo_path)
                    } else {
                        let e = &md.first_seen[0];
                        Provenance::from_git_repo_and_commit(
                            repo_path,
                            CommitKind::FirstSeen,
                            e.commit_oid,
                            e.path.clone(),
                        )
                    };

                    run_matcher(
                        matcher,
                        provenance,
                        blob,
                        &send_ds,
                        args.snippet_length,
                        args.metadata_args.blob_metadata,
                        &progress,
                    )?;

                    Ok(())
                },
            )
        })?;

    // ---------------------------------------------------------------------------------------------
    // Wait for all inputs to be scanned and the database thread to finish
    // ---------------------------------------------------------------------------------------------
    // Get rid of the reference to the sending channel after starting the scanners,
    // to ensure things terminate as expected.
    drop(send_ds);
    let (datastore, num_matches, num_new_matches) = datastore_writer.join().unwrap();
    progress.finish();

    // ---------------------------------------------------------------------------------------------
    // Finalize and report
    // ---------------------------------------------------------------------------------------------
    {
        debug!("{} matchers created during scan", num_matchers_counter.into_inner()?);
        debug!("{} items in the blob ID set", seen_blobs.len());

        let matcher_stats = matcher_stats.into_inner()?;
        let scan_duration = scan_start.elapsed();
        let seen_bytes_per_sec =
            (matcher_stats.bytes_seen as f64 / scan_duration.as_secs_f64()) as u64;

        println!(
            "Scanned {} from {} blobs in {} ({}/s); {}/{} new matches",
            HumanBytes(matcher_stats.bytes_seen),
            HumanCount(matcher_stats.blobs_seen),
            HumanDuration(scan_duration),
            HumanBytes(seen_bytes_per_sec),
            HumanCount(num_new_matches),
            HumanCount(num_matches),
        );

        #[cfg(feature = "rule_profiling")]
        {
            println!("Rule stats:");
            let mut entries = matcher_stats.rule_stats.get_entries();
            entries.retain(|e| e.raw_match_count > 0);
            entries.sort_by_key(|e| e.stage2_duration);
            entries.reverse();
            for entry in entries {
                let rule_name = &rules_db
                    .get_rule(entry.rule_id)
                    .expect("rule index should be valid")
                    .name;
                println!(
                    "{:>50} {:>10} {:>10.4}s",
                    rule_name,
                    entry.raw_match_count,
                    entry.stage2_duration.as_secs_f64()
                );
            }
        }

        if num_matches > 0 {
            let matches_summary = datastore.summarize()?;
            let matches_table = crate::cmd_summarize::summary_table(&matches_summary);
            println!();
            matches_table.print_tty(global_args.use_color())?;
        }

        println!("\nRun the `report` command next to show finding details.");
    }

    Ok(())
}

#[derive(Default)]
struct MetadataResult {
    mime_essence: Option<String>,
    charset: Option<String>,
}

impl MetadataResult {
    fn from_blob_and_provenance(
        guesser: &Guesser,
        blob: &Blob,
        provenance: &Provenance,
    ) -> MetadataResult {
        let input = match &provenance {
            Provenance::File(e) => {
                content_guesser::Input::from_path_and_bytes(&e.path, &blob.bytes)
            }
            Provenance::GitRepo(e) => match &e.commit_provenance {
                None => content_guesser::Input::from_bytes(&blob.bytes),
                Some(md) => match md.blob_path.to_path() {
                    Ok(blob_path) => {
                        content_guesser::Input::from_path_and_bytes(blob_path, &blob.bytes)
                    }
                    Err(_e) => content_guesser::Input::from_bytes(&blob.bytes),
                },
            },
        };
        // let path: Option<std::path::PathBuf> = input.path().map(|p| p.to_owned());
        let guess = guesser.guess(input);
        let (mime_essence, charset) = match guess.best_guess() {
            None => (None, None),
            Some(m) => {
                let essence = m.essence_str().to_owned();
                let charset = m.get_param(mime::CHARSET).map(|n| n.to_string());
                (Some(essence), charset)
            }
        };
        // progress.suspend(|| {
        //     info!("{}:  {:?}:  {:?}:  {:?}",
        //           blob.id,
        //           path,
        //           essence, charset);
        // });
        MetadataResult {
            mime_essence,
            charset,
        }
    }
}

fn run_matcher(
    matcher_guesser: &mut (Matcher, Guesser),
    provenance: Provenance,
    blob: Blob,
    send_ds: &crossbeam_channel::Sender<(BlobMetadata, Vec<Match>)>,
    snippet_length: usize,
    blob_metadata_recording_mode: args::BlobMetadataMode,
    progress: &Progress,
) -> Result<()> {
    let (matcher, guesser) = matcher_guesser;

    let matches = match matcher.scan_blob(&blob, &provenance) {
        Err(e) => {
            progress.suspend(|| error!("Failed to scan blob {} from {provenance}: {e}", blob.id));
            return Ok(());
        }
        Ok(v) => v,
    };

    if blob_metadata_recording_mode != args::BlobMetadataMode::All && matches.is_empty() {
        return Ok(());
    }

    let metadata = {
        let (mime_essence, charset) = match blob_metadata_recording_mode {
            args::BlobMetadataMode::None => (None, None),
            _ => {
                let md = MetadataResult::from_blob_and_provenance(&guesser, &blob, &provenance);
                (md.mime_essence, md.charset)
            }
        };
        BlobMetadata {
            id: blob.id,
            num_bytes: blob.len(),
            mime_essence,
            charset,
        }
    };

    // Convert each BlobMatch into a regular Match
    let matches = match matches
        .iter()
        .map(|m| m.matching_input_offset_span.end)
        .max()
    {
        Some(max_end) => {
            // compute the location mapping only on the input that's necessary to look at
            let loc_mapping = location::LocationMapping::new(&blob.bytes[0..max_end]);

            let capacity: usize = matches.iter().map(|m| m.captures.len() - 1).sum();
            let mut new_matches = Vec::with_capacity(capacity);
            new_matches.extend(
                matches
                    .iter()
                    .map(|m| Match::convert(&loc_mapping, m, snippet_length))
                    .flatten(),
            );
            new_matches
        }
        None => Vec::new(),
    };

    send_ds.send((metadata, matches))?;

    Ok(())
}
