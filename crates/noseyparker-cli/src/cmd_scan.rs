use anyhow::{bail, Context, Result};
use indicatif::{HumanBytes, HumanCount, HumanDuration};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, debug_span, error, info, trace, warn};

use crate::{args, rule_loader::RuleLoader};

use content_guesser::Guesser;
use input_enumerator::{open_git_repo, FilesystemEnumerator, FoundInput};
use progress::Progress;

use noseyparker::blob::{Blob, BlobId};
use noseyparker::blob_id_map::BlobIdMap;
use noseyparker::blob_metadata::BlobMetadata;
use noseyparker::datastore::Datastore;
use noseyparker::defaults::DEFAULT_IGNORE_RULES;
use noseyparker::git_binary::{CloneMode, Git};
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
    #[cfg(feature = "github")]
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
        let mut repo_urls = args.input_specifier_args.git_url.clone();

        #[cfg(feature = "github")]
        {
            use noseyparker::github;

            let repo_specifiers = github::RepoSpecifiers {
                user: args.input_specifier_args.github_user.clone(),
                organization: args.input_specifier_args.github_organization.clone(),
                all_organizations: args.input_specifier_args.all_github_organizations,
                repo_filter: args.input_specifier_args.github_repo_type.into(),
            };

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
                    global_args.ignore_certs,
                    Some(&mut progress),
                )
                .context("Failed to enumerate GitHub repositories")?
                {
                    use noseyparker::git_url::GitUrl;
                    use std::str::FromStr;
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
        }

        repo_urls.sort();
        repo_urls.dedup();

        repo_urls
    };

    // ---------------------------------------------------------------------------------------------
    // Clone or update all mentioned Git URLs
    // ---------------------------------------------------------------------------------------------
    let mut input_roots = args.input_specifier_args.path_inputs.clone();

    if !repo_urls.is_empty() {
        info!("{} Git URLs to fetch", repo_urls.len());
        for repo_url in &repo_urls {
            debug!("Need to fetch {repo_url}")
        }

        let clone_mode = match args.input_specifier_args.git_clone {
            args::GitCloneMode::Mirror => CloneMode::Mirror,
            args::GitCloneMode::Bare => CloneMode::Bare,
        };
        let git = Git::new(global_args.ignore_certs);

        let mut progress =
            Progress::new_bar(repo_urls.len() as u64, "Fetching Git repos", progress_enabled);

        for repo_url in repo_urls {
            progress.set_message(format!("Fetching Git repos ({repo_url})"));

            let output_dir = match datastore.clone_destination(&repo_url) {
                Err(e) => {
                    progress.suspend(|| {
                        error!("Failed to determine output directory for {repo_url}: {e}; skipping scan");
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
                    error!(
                        "Failed to clone {repo_url} to {}: {e}; skipping scan",
                        output_dir.display()
                    );
                });
                progress.inc(1);
                continue;
            }
            input_roots.push(output_dir);
            progress.inc(1);
        }

        progress.finish_with_message("Fetching Git repos");
    }

    if input_roots.is_empty() && args.input_specifier_args.enumerators.is_empty() {
        bail!("No inputs to scan");
    }

    // ---------------------------------------------------------------------------------------------
    // Enumerate inputs and scan
    // ---------------------------------------------------------------------------------------------
    let scan_start = Instant::now();

    // Kick off enumerator in a separate thread, writing results to a channel, so that scanning can
    // proceed concurrently
    let (input_enumerator_thread, input_recv) =
        make_input_enumerator_thread(&args, &datastore, input_roots)?;

    // ---------------------------------------------------------------------------------------------
    // Define some matcher helper code and shared state
    // ---------------------------------------------------------------------------------------------
    let num_blob_processors = Mutex::new(0u64); // how many blob processors have been initialized?
    let matcher_stats = Mutex::new(MatcherStats::default());
    let seen_blobs = BlobIdMap::new();

    // FIXME: have this print out aggregate rate at finish
    let mut progress = Progress::new_bytes_spinner("Scanning content", progress_enabled);

    // FIXME: expose the following as a CLI parameter
    const DATASTORE_BATCH_SIZE: usize = 16 * 1024;
    const DATASTORE_COMMIT_INTERVAL: Duration = Duration::from_secs(1);

    // Create a channel pair for processor threads to get their results to the datastore recorder.
    let (send_ds, recv_ds) = {
        let channel_size =
            std::cmp::max(args.num_jobs * DATASTORE_BATCH_SIZE, 64 * DATASTORE_BATCH_SIZE);
        crossbeam_channel::bounded::<DatastoreMessage>(channel_size)
    };

    let blobs_dir = datastore.blobs_dir();

    let t1 = Instant::now();
    let matcher = Matcher::new(&rules_db, &seen_blobs, Some(&matcher_stats))?;
    let blob_processor_init_time = Mutex::new(t1.elapsed());
    let make_blob_processor = || -> Result<BlobProcessor> {
        let t1 = Instant::now();
        let matcher = matcher.clone();
        *num_blob_processors.lock().unwrap() += 1;
        let guesser = Guesser::new()?;
        {
            let mut init_time = blob_processor_init_time.lock().unwrap();
            *init_time = *init_time + t1.elapsed();
        }

        Ok(BlobProcessor {
            matcher,
            guesser,
            progress: &progress,
            blobs_dir: &blobs_dir,
            snippet_length: args.snippet_length,
            blob_metadata_recording_mode: args.metadata_args.blob_metadata,
            copy_blobs: args.copy_blobs,
            send_ds: &send_ds,
        })
    };

    // ---------------------------------------------------------------------------------------------
    // Create datastore writer thread.
    // The datastore uses SQLite, which does best with a single writer.
    // ---------------------------------------------------------------------------------------------
    let datastore_writer_thread = std::thread::Builder::new()
        .name("datastore".to_string())
        .spawn(move || -> Result<_> {
            let _span = debug_span!("datastore", "{}", datastore.root_dir().display()).entered();
            let mut total_recording_time: std::time::Duration = Default::default();

            let mut num_matches_added: u64 = 0;
            let mut total_messages: u64 = 0;

            // Big idea: read until all the senders hang up; panic if recording matches fails.
            //
            // Record all messages chunked transactions, trying to commit at least every
            // DATASTORE_COMMIT_INTERVAL.

            let mut batch: Vec<DatastoreMessage> = Vec::with_capacity(DATASTORE_BATCH_SIZE);
            let mut matches_in_batch: usize = 0;
            let mut last_commit_time = Instant::now();

            for message in recv_ds {
                total_messages += 1;
                matches_in_batch += message.2.len();
                batch.push(message);

                if batch.len() >= DATASTORE_BATCH_SIZE
                    || matches_in_batch >= DATASTORE_BATCH_SIZE
                    || last_commit_time.elapsed() >= DATASTORE_COMMIT_INTERVAL
                {
                    let t1 = std::time::Instant::now();
                    let batch_len = batch.len();
                    let tx = datastore.begin()?;
                    let num_added = tx
                        .record(batch.as_slice())
                        .context("Failed to record batch")?;
                    tx.commit()?;
                    last_commit_time = Instant::now();
                    num_matches_added += num_added;
                    batch.clear();
                    matches_in_batch = 0;
                    let elapsed = t1.elapsed();
                    trace!(
                        "Recorded {num_added} matches from {batch_len} messages in {:.6}s",
                        elapsed.as_secs_f64()
                    );
                    total_recording_time += elapsed;
                }
            }

            if !batch.is_empty() {
                let t1 = std::time::Instant::now();

                let batch_len = batch.len();
                let tx = datastore.begin()?;
                let num_added = tx
                    .record(batch.as_slice())
                    .context("Failed to record batch")?;
                tx.commit()?;
                num_matches_added += num_added;
                // batch.clear();
                // matches_in_batch = 0;

                let elapsed = t1.elapsed();
                trace!(
                    "Recorded {num_added} matches from {batch_len} messages in {:.6}s",
                    elapsed.as_secs_f64()
                );
                total_recording_time += elapsed;
            }

            let num_matches = datastore.get_num_matches()?;
            let t1 = std::time::Instant::now();
            datastore.analyze()?;
            let analyzed_elapsed = t1.elapsed();

            debug!(
                "Summary: recorded {num_matches} matches from {total_messages} messages \
                     in {:.6}s; analyzed in {:.6}s",
                total_recording_time.as_secs_f64(),
                analyzed_elapsed.as_secs_f64()
            );

            Ok((datastore, num_matches, num_matches_added))
        })?;

    // ---------------------------------------------------------------------------------------------
    // Scan enumerated inputs
    //
    // Kick off scanner threads, but for better error messages, don't check its result until after
    // checking the datastore writer thread.
    // ---------------------------------------------------------------------------------------------
    let scan_res: Result<()> = input_recv
        .into_iter()
        .par_bridge()
        .try_for_each_init(
            || -> Result<(BlobProcessor<'_>, Progress)> {
                let processor = make_blob_processor()?;
                Ok((processor, progress.clone()))
            },
            move |state: &mut Result<_>, found_input: FoundInput| -> Result<()> {
                let (processor, progress): &mut (BlobProcessor<'_>, Progress) = match state {
                    Ok(state) => state,
                    Err(e) => bail!("Failed to initialize worker: {e}"),
                };
                match found_input {
                    FoundInput::File(file_result) => {
                        let _span = debug_span!("file-scan", "{}", file_result.path.display())
                            .entered();

                        let fname = &file_result.path;
                        let blob = match Blob::from_file(fname) {
                            Err(e) => {
                                error!("Failed to load blob from {}: {}", fname.display(), e);
                                return Ok(());
                            }
                            Ok(v) => v,
                        };
                        progress.inc(file_result.num_bytes);

                        processor.run(
                            Provenance::from_file(fname.clone()).into(),
                            blob,
                        )?;

                        Ok(())
                    }

                    FoundInput::GitRepo(git_repo_result) => {
                        let span = debug_span!("git-scan", "{}", git_repo_result.path.display());
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

                        git_repo_result
                            .blobs
                            .into_par_iter()
                            // XXX try to be more conservative with parallelism here; use
                            // explicitly larger granularity.
                            //
                            // Git repos are typically represented with packfiles on disk, and
                            // oftentimes with just a single packfile.
                            //
                            // gix _does_ allow packfiles to be read by multiple threads with
                            // decent parallel speedup up to a few threads, but it doesn't scale
                            // linearly.
                            //
                            // The optimal efficiency for reading all blobs from a Git repo would
                            // probably involve one thread per packfile. Doing that would require
                            // restructuring this code.
                            .with_min_len(512)
                            .try_for_each_init(
                                || -> Result<_> {
                                    let _span = span.enter();
                                    let repo = repository.to_thread_local();
                                    let processor = make_blob_processor()?;
                                    Ok((repo, processor, progress.clone()))
                                },
                                |state: &mut Result<_>, md| -> Result<()> {
                                    let _span = span.enter();
                                    let (repo, processor, progress) = match state {
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

                                    let provenance = ProvenanceSet::try_from_iter(
                                        md.first_seen
                                            .iter()
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
                                            }))
                                        .unwrap_or_else(|| Provenance::from_git_repo(repo_path.clone()).into() );

                                    processor.run(
                                        provenance,
                                        blob,
                                    )?;
                                    Ok(())
                                }
                        )?;

                        Ok(())
                    }

                    FoundInput::EnumeratorBlob(enum_result) => {
                        progress.inc(enum_result.content.as_bytes().len().try_into().unwrap());
                        let blob = Blob::from_bytes(enum_result.content.as_bytes().to_owned());

                        let _span = debug_span!("enum-scan-scan", "{}", blob.id)
                            .entered();

                        debug!("Got blob from enumerator: {} bytes: {:?}", blob.len(), enum_result.provenance);

                        processor.run(
                            Provenance::from_extended(enum_result.provenance).into(),
                            blob,
                        )?;

                        Ok(())
                    }
                }
            });

    // ---------------------------------------------------------------------------------------------
    // Close any open channel ends to allow everything to terminate
    // ---------------------------------------------------------------------------------------------
    drop(send_ds);

    // ---------------------------------------------------------------------------------------------
    // Wait for all inputs to be enumerated and scanned and the database thread to finish
    // ---------------------------------------------------------------------------------------------
    input_enumerator_thread
        .join()
        .unwrap()
        .context("Failed to enumerate inputs")?;

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
        debug!(
            "{} blob processors created in {:.1}s during scan",
            num_blob_processors.into_inner()?,
            blob_processor_init_time.into_inner()?.as_secs_f64()
        );
        debug!("{} items in the blob ID set", seen_blobs.len());

        drop(matcher);
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
            let summary = datastore
                .get_summary()
                .context("Failed to get finding summary")
                .unwrap();
            let table = crate::cmd_summarize::summary_table(&summary);
            println!();
            table.print_tty(global_args.use_color(std::io::stdout()))?;
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

/// A combined matcher, content type guesser, and a number of parameters that don't change within
/// one `scan` run
struct BlobProcessor<'a> {
    matcher: Matcher<'a>,
    guesser: Guesser,

    send_ds: &'a crossbeam_channel::Sender<DatastoreMessage>,
    snippet_length: usize,
    blob_metadata_recording_mode: args::BlobMetadataMode,
    copy_blobs: args::CopyBlobsMode,
    blobs_dir: &'a Path,
    progress: &'a Progress,
}

impl<'a> BlobProcessor<'a> {
    #[allow(clippy::too_many_arguments)]
    fn run(&mut self, provenance: ProvenanceSet, blob: Blob) -> Result<()> {
        let blob_id = blob.id.hex();
        let _span = debug_span!("matcher", blob_id).entered();

        let t1 = Instant::now();
        let res = self.matcher.scan_blob(&blob, &provenance);
        let scan_time = t1.elapsed();
        let scan_us = scan_time.as_micros();

        match res {
            Err(e) => {
                self.progress.suspend(|| {
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
                self.send_ds
                    .send((provenance, metadata, Vec::new()))
                    .context("Failed to save blob scan results")?;
                Ok(())
            }

            // blob has not been seen; need to record blob metadata, provenance, and matches
            Ok(ScanResult::New(matches)) => {
                trace!("({scan_us}us) blob newly scanned; {} matches", matches.len());

                let do_copy_blob = match self.copy_blobs {
                    args::CopyBlobsMode::All => true,
                    args::CopyBlobsMode::Matching => !matches.is_empty(),
                    args::CopyBlobsMode::None => false,
                };
                if do_copy_blob {
                    let output_dir = self.blobs_dir.join(&blob_id[..2]);
                    let output_path = output_dir.join(&blob_id[2..]);
                    trace!("saving blob to {}", output_path.display());
                    match std::fs::create_dir(&output_dir) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(e) => {
                            bail!(
                                "Failed to create blob directory at {}: {}",
                                output_dir.display(),
                                e
                            );
                        }
                    }
                    std::fs::write(&output_path, &blob.bytes).with_context(|| {
                        format!("Failed to write blob contents to {}", output_path.display())
                    })?;
                }

                // If there are no matches, we can bail out here and avoid recording anything.
                // UNLESS the `--blob-metadata=all` mode was specified; then we need to record the
                // provenance for _all_ seen blobs.
                if self.blob_metadata_recording_mode != args::BlobMetadataMode::All
                    && matches.is_empty()
                {
                    return Ok(());
                }

                let metadata = match self.blob_metadata_recording_mode {
                    args::BlobMetadataMode::None => BlobMetadata {
                        id: blob.id,
                        num_bytes: blob.len(),
                        mime_essence: None,
                        charset: None,
                    },
                    _ => {
                        let md = MetadataResult::from_blob_and_provenance(
                            &self.guesser,
                            &blob,
                            &provenance,
                        );
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
                            matches.iter().map(|m| {
                                (None, Match::convert(&loc_mapping, m, self.snippet_length))
                            }),
                        );
                        new_matches
                    }
                    None => {
                        debug_assert!(matches.is_empty());
                        Vec::new()
                    }
                };

                self.send_ds
                    .send((provenance, metadata, matches))
                    .context("Failed to save results")?;
                Ok(())
            }
        }
    }
}

/// Initialize a `FilesystemEnumerator` based on the command-line arguments and datastore.
fn make_fs_enumerator(
    args: &args::ScanArgs,
    datastore: &Datastore,
    input_roots: Vec<PathBuf>,
) -> Result<Option<FilesystemEnumerator>> {
    if input_roots.is_empty() {
        Ok(None)
    } else {
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

        Ok(Some(ie))
    }
}

/// Create a separate thread for enumerating the inputs specified by command-line arguments and
/// datastore.
fn make_input_enumerator_thread(
    args: &args::ScanArgs,
    datastore: &Datastore,
    input_roots: Vec<PathBuf>,
) -> Result<(std::thread::JoinHandle<Result<()>>, crossbeam_channel::Receiver<FoundInput>)> {
    let fs_enumerator = make_fs_enumerator(args, datastore, input_roots)
        .context("Failed to initialize filesystem enumerator")?;

    let num_jobs = args.num_jobs;

    // Create a pair of channels for the input enumeration
    let (input_send, input_recv) = {
        let channel_size = std::cmp::max(num_jobs * 32, 256);
        crossbeam_channel::bounded(channel_size)
    };

    let enumerators = args.input_specifier_args.enumerators.clone();

    let input_enumerator_thread = std::thread::Builder::new()
        .name("input_enumerator".to_string())
        .spawn(move || -> Result<_> {
            // Find inputs from disk first. This is parallelized internally in the `.run()` method.
            if let Some(fs_enumerator) = fs_enumerator {
                fs_enumerator.run(input_send.clone())?;
            }

            // Find inputs from enumerator files in parallel.
            //
            // The parallelism approach:
            //
            // - Parallelize across all enumerators
            //
            // - For a single enumerator:
            //   - Split into lines sequentially
            //   - Parallelize JSON deserialization (JSON is an expensive serialization format, but
            //     easy to sling around, hence used here -- another format like Arrow or msgpack
            //     would be much more efficient)
            //   - Send deserialized items on the `input_send` channel
            //
            // - Do all the parallel work in a separate Rayon threadpool to avoid deadlocks from
            //   the `input_send` channel
            //
            // An alternative would be to read each file's lines in parallel, but this is difficult
            // to implement efficiently in a way that gets parallel speedup.
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_jobs)
                .use_current_thread()
                .build_scoped(
                    |thread| thread.run(),
                    |pool| {
                        pool.install(move || {
                            enumerators.into_par_iter().for_each(|ef| {
                                if let Err(e) = rayon_read_from_enumerator(&ef, input_send.clone())
                                {
                                    error!("Failed to read from enumerator {}: {e}", ef.display());
                                }
                            })
                        })
                    },
                )?;

            Ok(())
        })
        .context("Failed to enumerate filesystem inputs")?;

    Ok((input_enumerator_thread, input_recv))
}

/// Read from a single enumerator file in parallel, sending inputs for scanning to `input_send`.
fn rayon_read_from_enumerator(
    fname: &Path,
    input_send: crossbeam_channel::Sender<FoundInput>,
) -> Result<()> {
    use std::io::BufRead;
    let file = std::fs::File::open(fname)?;
    let reader = std::io::BufReader::new(file);
    reader
        .lines()
        .zip(1usize..)
        .into_iter()
        .par_bridge()
        .filter_map(|(line, line_num)| line.map(|line| (line, line_num)).ok())
        .for_each(|(line, line_num)| match serde_json::from_str(&line) {
            Ok(e) => input_send.send(FoundInput::EnumeratorBlob(e)).unwrap(),
            Err(e) => {
                error!("Error on enumerator {}:{line_num}: {e}", fname.display());
            }
        });
    Ok(())
}
