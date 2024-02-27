use anyhow::{bail, Context, Result};
use indicatif::{HumanBytes, HumanCount, HumanDuration};
use rayon::prelude::*;
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{debug, debug_span, error, info, trace, trace_span, warn};

use crate::{args, rule_loader::RuleLoader};

use content_guesser::Guesser;
use input_enumerator::{open_git_repo, FileResult, FilesystemEnumerator};
use progress::Progress;

use noseyparker::blob::{Blob, BlobId};
use noseyparker::blob_id_map::BlobIdMap;
use noseyparker::blob_metadata::BlobMetadata;
use noseyparker::datastore::Datastore;
use noseyparker::defaults::DEFAULT_IGNORE_RULES;
use noseyparker::git_binary::{CloneMode, Git};
use noseyparker::git_url::GitUrl;
use noseyparker::github;
use noseyparker::location;
use noseyparker::match_type::Match;
use noseyparker::matcher::{Matcher, ScanResult};
use noseyparker::matcher_stats::MatcherStats;
use noseyparker::provenance::Provenance;
use noseyparker::provenance_set::ProvenanceSet;
use noseyparker::rules_database::RulesDatabase;

type DatastoreMessage = (ProvenanceSet, BlobMetadata, Vec<(Option<f64>, Match)>);

/// This command scans multiple filesystem inputs for secrets.
/// The implementation enumerates content in parallel, scans the enumerated content in parallel,
/// and records found matches to a SQLite database from a single dedicated thread.
pub fn run(global_args: &args::GlobalArgs, args: &args::ScanArgs) -> Result<()> {
    args::validate_github_api_url(
        &args.input_specifier_args.github_api_url,
        args.input_specifier_args.all_github_organizations,
    );

    debug!("Args:\n{global_args:#?}\n{args:#?}");

    let progress_enabled = global_args.use_progress();
    let mut init_progress = Progress::new_spinner("Initializing...", progress_enabled);

    // ---------------------------------------------------------------------------------------------
    // Configure the Rayon global thread pool
    // ---------------------------------------------------------------------------------------------
    init_progress.set_message("Initializing thread pool...");
    rayon::ThreadPoolBuilder::new()
        .num_threads(args.num_jobs)
        .thread_name(|idx| format!("scanner-{idx}"))
        .build_global()
        .context("Failed to initialize Rayon")?;

    // ---------------------------------------------------------------------------------------------
    // Open datastore
    // ---------------------------------------------------------------------------------------------
    init_progress.set_message("Initializing datastore...");
    let mut datastore =
        Datastore::create_or_open(&args.datastore, global_args.advanced.sqlite_cache_size)
            .with_context(|| {
                format!("Failed to open datastore at {}", &args.datastore.display())
            })?;

    // ---------------------------------------------------------------------------------------------
    // Load rules
    // ---------------------------------------------------------------------------------------------
    init_progress.set_message("Compiling rules...");
    let rules_db = {
        let loaded = RuleLoader::from_rule_specifiers(&args.rules)
            .load()
            .context("Failed to load rules")?;
        let resolved = loaded
            .resolve_enabled_rules()
            .context("Failed to resolve rules")?;
        RulesDatabase::from_rules(resolved.into_iter().cloned().collect())
            .context("Failed to compile rules")?
    };

    // ---------------------------------------------------------------------------------------------
    // Record rules to the datastore
    // ---------------------------------------------------------------------------------------------
    init_progress.set_message("Recording rules...");
    let mut record_rules = || -> Result<()> {
        let tx = datastore.begin()?;
        tx.record_rules(rules_db.rules())?;
        tx.commit()?;
        Ok(())
    };
    record_rules().context("Failed to record rules to the datastore")?;

    drop(init_progress);

    // ---------------------------------------------------------------------------------------------
    // Enumerate any mentioned GitHub repositories; gather list of all repos to clone or update
    // ---------------------------------------------------------------------------------------------
    let repo_urls = {
        let repo_specifiers = github::RepoSpecifiers {
            user: args.input_specifier_args.github_user.clone(),
            organization: args.input_specifier_args.github_organization.clone(),
            all_organizations: args.input_specifier_args.all_github_organizations,
        };
        let mut repo_urls = args.input_specifier_args.git_url.clone();
        if !repo_specifiers.is_empty() {
            let mut progress = Progress::new_countup_spinner(
                "Enumerating GitHub repositories...",
                progress_enabled,
            );
            let mut num_found: u64 = 0;
            let api_url = args.input_specifier_args.github_api_url.clone();

            for repo_string in github::enumerate_repo_urls(
                &repo_specifiers,
                api_url,
                args.ignore_certs,
                Some(&mut progress),
            )
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
        let clone_mode = match args.input_specifier_args.git_clone {
            args::GitCloneMode::Mirror => CloneMode::Mirror,
            args::GitCloneMode::Bare => CloneMode::Bare,
        };
        let git = Git::new(args.ignore_certs);

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
            if args.input_specifier_args.git_history == args::GitHistoryMode::None {
                ie.enumerate_git_history(false);
            }

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
            "Found {} from {} plain {} and {} blobs from {} Git {}",
            HumanBytes(total_bytes_found),
            HumanCount(inputs.files.len() as u64),
            if inputs.files.len() == 1 {
                "file"
            } else {
                "files"
            },
            HumanCount(inputs.git_repos.iter().map(|r| r.num_blobs()).sum()),
            HumanCount(inputs.git_repos.len() as u64),
            if inputs.git_repos.len() == 1 {
                "repo"
            } else {
                "repos"
            },
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
    let seen_blobs = BlobIdMap::new();

    let make_matcher = || -> Result<(Matcher, Guesser)> {
        *num_matchers_counter.lock().unwrap() += 1;
        let matcher = Matcher::new(&rules_db, &seen_blobs, Some(&matcher_stats))?;
        let guesser = Guesser::new()?;
        Ok((matcher, guesser))
    };

    // FIXME: have this print out aggregate rate at finish
    let mut progress =
        Progress::new_bytes_bar(total_blob_bytes, "Scanning content", progress_enabled);

    // FIXME: expose the following as a CLI parameter
    const BATCH_SIZE: usize = 16 * 1024;

    // Create a channel pair for matcher threads to get their results to the datastore recorder.
    let channel_size = std::cmp::max(args.num_jobs * BATCH_SIZE, 64 * BATCH_SIZE);
    let (send_ds, recv_ds) = crossbeam_channel::bounded::<DatastoreMessage>(channel_size);

    let blobs_dir = datastore.blobs_dir();

    // We create a separate thread for writing matches to the datastore.
    // The datastore uses SQLite, which does best with a single writer.
    let datastore_writer_thread = std::thread::Builder::new()
        .name("datastore".to_string())
        .spawn(move || -> Result<_> {
            let _span = debug_span!("datastore").entered();
            let mut total_recording_time: std::time::Duration = Default::default();

            let mut num_matches_added: u64 = 0;
            let mut total_messages: u64 = 0;

            // Big idea: read until all the senders hang up; panic if recording matches fails.
            //
            // Record all messages in one big transaction to maximize throughput.

            let mut batch: Vec<DatastoreMessage> = Vec::with_capacity(BATCH_SIZE);
            let mut matches_in_batch: usize = 0;

            let tx = datastore.begin()?;

            for message in recv_ds {
                total_messages += 1;
                matches_in_batch += message.2.len();
                batch.push(message);

                if batch.len() >= BATCH_SIZE || matches_in_batch >= BATCH_SIZE {
                    let t1 = std::time::Instant::now();
                    let batch_len = batch.len();
                    let num_added = tx
                        .record(batch.as_slice())
                        .context("Failed to record batch")?;
                    num_matches_added += num_added;
                    batch.clear();
                    matches_in_batch = 0;
                    let elapsed = t1.elapsed();
                    debug!(
                        "Recorded {num_added} matches from {batch_len} messages in {:.6}s",
                        elapsed.as_secs_f64()
                    );
                    total_recording_time += elapsed;
                }
            }

            if !batch.is_empty() {
                let t1 = std::time::Instant::now();

                let batch_len = batch.len();
                let num_added = tx
                    .record(batch.as_slice())
                    .context("Failed to record batch")?;
                num_matches_added += num_added;
                // batch.clear();
                // matches_in_batch = 0;

                let elapsed = t1.elapsed();
                debug!(
                    "Recorded {num_added} matches from {batch_len} messages in {:.6}s",
                    elapsed.as_secs_f64()
                );
                total_recording_time += elapsed;
            }

            let t1 = std::time::Instant::now();
            tx.commit()?;
            let commit_elapsed = t1.elapsed();

            let num_matches = datastore.get_num_matches()?;

            let t1 = std::time::Instant::now();
            datastore.analyze()?;
            let analyzed_elapsed = t1.elapsed();

            debug!(
                "Summary: recorded {num_matches} matches from {total_messages} messages \
                     in {:.6}s; committed in {:.6}s; analyzed in {:.6}s",
                total_recording_time.as_secs_f64(),
                commit_elapsed.as_secs_f64(),
                analyzed_elapsed.as_secs_f64()
            );

            Ok((datastore, num_matches, num_matches_added))
        })?;

    // A function to be immediately called, to allow syntactic simplification of error propagation
    let scan_inner = || -> Result<()> {
        // ---------------------------------------------------------------------------------------------
        // Scan plain files
        // ---------------------------------------------------------------------------------------------
        inputs.files.par_iter().try_for_each_init(
            || -> Result<_> {
                let matcher = make_matcher()?;

                Ok((matcher, progress.clone()))
            },
            |state: &mut Result<_>, file_result: &FileResult| -> Result<()> {
                let _span = trace_span!("file-scan", path = file_result.path.display().to_string())
                    .entered();

                let (matcher, progress) = match state {
                    Ok(state) => state,
                    Err(e) => bail!("Failed to initialize worker: {e}"),
                };

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
                    ProvenanceSet::new(Provenance::from_file(fname.clone()), Vec::new()),
                    blob,
                    &send_ds,
                    args.snippet_length,
                    args.metadata_args.blob_metadata,
                    args.copy_blobs,
                    &blobs_dir,
                    progress,
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
                let span =
                    trace_span!("git-scan", path = git_repo_result.path.display().to_string());
                let _span = span.enter();

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
                    || -> Result<_> {
                        let _span = span.enter();
                        let repo = repository.to_thread_local();
                        let matcher = make_matcher()?;
                        Ok((repo, matcher, progress.clone()))
                    },
                    |state: &mut Result<_>, md| -> Result<()> {
                        let _span = span.enter();
                        let (repo, matcher, progress) = match state {
                            Ok(state) => state,
                            Err(e) => bail!("Failed to initialize worker: {e}"),
                        };

                        let size = md.num_bytes;
                        let blob_id = md.blob_oid;
                        progress.inc(size);
                        let repo_path = &git_repo_result.path;

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

                        let provenance = {
                            let mut it = md.first_seen.iter();
                            if let Some(e) = it.next() {
                                let commit_metadata = git_repo_result
                                    .commit_metadata
                                    .get(&e.commit_oid)
                                    .expect("should have commit metadata");
                                let p = Provenance::from_git_repo_with_first_commit(
                                    repo_path.clone(),
                                    commit_metadata.clone(),
                                    e.path.clone(),
                                );

                                let ps = it
                                    .map(|e| {
                                        let commit_metadata = git_repo_result
                                            .commit_metadata
                                            .get(&e.commit_oid)
                                            .expect("should have commit metadata");
                                        Provenance::from_git_repo_with_first_commit(
                                            repo_path.clone(),
                                            commit_metadata.clone(),
                                            e.path.clone(),
                                        )
                                    })
                                    .collect();

                                ProvenanceSet::new(p, ps)
                            } else {
                                ProvenanceSet::new(
                                    Provenance::from_git_repo(repo_path.clone()),
                                    Vec::new(),
                                )
                            }
                        };

                        run_matcher(
                            matcher,
                            provenance,
                            blob,
                            &send_ds,
                            args.snippet_length,
                            args.metadata_args.blob_metadata,
                            args.copy_blobs,
                            &blobs_dir,
                            progress,
                        )?;

                        Ok(())
                    },
                )
            })?;

        Ok(())
    };

    // kick off scanner threads, but for better error messages, don't return its error until after
    // checking the datastore writer thread
    let scan_res = scan_inner();

    // ---------------------------------------------------------------------------------------------
    // Wait for all inputs to be scanned and the database thread to finish
    // ---------------------------------------------------------------------------------------------
    // Get rid of the reference to the sending channel after starting the scanners,
    // to ensure things terminate as expected.
    drop(send_ds);
    let (datastore, num_matches, num_new_matches) = datastore_writer_thread
        .join()
        .unwrap()
        .context("Failed to save results to the datastore")?;
    progress.finish();

    // now check the result of the scanners
    scan_res.context("Failed to scan inputs")?;

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
                    .name();
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
            matches_table.print_tty(global_args.use_color(std::io::stdout()))?;
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
        provenance: &ProvenanceSet,
    ) -> MetadataResult {
        let blob_path: Option<&'_ Path> = provenance.iter().find_map(|p| p.blob_path());
        let input = match blob_path {
            None => content_guesser::Input::from_bytes(&blob.bytes),
            Some(blob_path) => content_guesser::Input::from_path_and_bytes(blob_path, &blob.bytes),
        };

        let guess = guesser.guess(input);
        match guess.best_guess() {
            None => MetadataResult::default(),
            Some(m) => MetadataResult {
                mime_essence: Some(m.essence_str().to_owned()),
                charset: m.get_param(mime::CHARSET).map(|n| n.to_string()),
            },
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_matcher(
    matcher_guesser: &mut (Matcher, Guesser),
    provenance: ProvenanceSet,
    blob: Blob,
    send_ds: &crossbeam_channel::Sender<DatastoreMessage>,
    snippet_length: usize,
    blob_metadata_recording_mode: args::BlobMetadataMode,
    copy_blobs: args::CopyBlobsMode,
    blobs_dir: &Path,
    progress: &Progress,
) -> Result<()> {
    let blob_id = blob.id.hex();
    let _span = trace_span!("matcher", blob_id = blob_id).entered();

    let (matcher, guesser) = matcher_guesser;

    let t1 = Instant::now();
    let res = matcher.scan_blob(&blob, &provenance);
    let scan_time = t1.elapsed();
    let scan_us = scan_time.as_micros();

    match res {
        Err(e) => {
            progress.suspend(|| {
                error!("Failed to scan blob {} from {}: {e}", blob.id, provenance.first())
            });
            Ok(())
        }

        // blob already seen, but with no matches; nothing to do!
        Ok(ScanResult::SeenSansMatches) => {
            trace!("({scan_us}us) blob already scanned with no matches");
            Ok(())
        }

        // blob already seen; all we need to do is record its provenance
        Ok(ScanResult::SeenWithMatches) => {
            trace!("({scan_us}us) blob already scanned with matches");
            let metadata = BlobMetadata {
                id: blob.id,
                num_bytes: blob.len(),
                mime_essence: None,
                charset: None,
            };
            send_ds
                .send((provenance, metadata, Vec::new()))
                .context("Failed to save blob scan results")?;
            Ok(())
        }

        // blob has not been seen; need to record blob metadata, provenance, and matches
        Ok(ScanResult::New(matches)) => {
            trace!("({scan_us}us) blob newly scanned; {} matches", matches.len());

            let do_copy_blob = match copy_blobs {
                args::CopyBlobsMode::All => true,
                args::CopyBlobsMode::Matching => !matches.is_empty(),
                args::CopyBlobsMode::None => false,
            };
            if do_copy_blob {
                let output_dir = blobs_dir.join(&blob_id[..2]);
                let output_path = output_dir.join(&blob_id[2..]);
                trace!("saving blob to {}", output_path.display());
                match std::fs::create_dir(&output_dir) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(e) => {
                        bail!("Failed to create blob directory at {}: {}", output_dir.display(), e);
                    }
                }
                std::fs::write(&output_path, &blob.bytes).with_context(|| {
                    format!("Failed to write blob contents to {}", output_path.display())
                })?;
            }

            if blob_metadata_recording_mode != args::BlobMetadataMode::All && matches.is_empty() {
                return Ok(());
            }

            let metadata = match blob_metadata_recording_mode {
                args::BlobMetadataMode::None => BlobMetadata {
                    id: blob.id,
                    num_bytes: blob.len(),
                    mime_essence: None,
                    charset: None,
                },
                _ => {
                    let md = MetadataResult::from_blob_and_provenance(guesser, &blob, &provenance);
                    BlobMetadata {
                        id: blob.id,
                        num_bytes: blob.len(),
                        mime_essence: md.mime_essence,
                        charset: md.charset,
                    }
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
                            .map(|m| (None, Match::convert(&loc_mapping, m, snippet_length))),
                    );
                    new_matches
                }
                None => {
                    debug_assert!(matches.is_empty());
                    Vec::new()
                }
            };

            send_ds
                .send((provenance, metadata, matches))
                .context("Failed to save results")?;
            Ok(())
        }
    }
}
