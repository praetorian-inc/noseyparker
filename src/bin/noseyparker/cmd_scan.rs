use anyhow::{Context, Result};
use indicatif::{HumanBytes, HumanCount, HumanDuration};
use rayon::prelude::*;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{debug, debug_span, error};

use crate::args;

use noseyparker::blob::{Blob, BlobId};
use noseyparker::blob_id_set::BlobIdSet;
use noseyparker::datastore::Datastore;
use noseyparker::defaults::DEFAULT_IGNORE_RULES;
use noseyparker::git2_utils;
use noseyparker::input_enumerator::{open_git_repo, FileResult, FilesystemEnumerator};
use noseyparker::location;
use noseyparker::match_type::Match;
use noseyparker::matcher::{BlobMatch, Matcher};
use noseyparker::matcher_stats::MatcherStats;
use noseyparker::progress::Progress;
use noseyparker::provenance::Provenance;
use noseyparker::rules_database::RulesDatabase;
use noseyparker::rules::Rules;

/// This command scans multiple filesystem inputs for secrets.
/// The implementation enumerates content in parallel, scans the enumerated content in parallel,
/// and records found matches to a SQLite database from a single dedicated thread.
pub fn run(global_args: &args::GlobalArgs, args: &args::ScanArgs) -> Result<()> {
    let _span = debug_span!("scan").entered();

    debug!("Args: {:?}", args);

    let color_enabled = global_args.use_color();
    let progress_enabled = global_args.use_progress();

    // ---------------------------------------------------------------------------------------------
    // Configure git2
    // ---------------------------------------------------------------------------------------------
    // From https://docs.rs/git2/latest/git2/opts/fn.enable_caching.html:
    //
    //     Controls whether or not libgit2 will cache loaded objects. Enabled by default, but
    //     disabling this can improve performance and memory usage if loading a large number of
    //     objects that will not be referenced again. Disabling this will cause repository objects
    //     to clear their caches when next accessed.
    git2::opts::enable_caching(false);

    // From https://docs.rs/git2/latest/git2/opts/fn.strict_hash_verification.html:
    //
    //     Controls whether or not libgit2 will verify that objects loaded have the expected hash.
    //     Enabled by default, but disabling this can significantly improve performance, at the
    //     cost of relying on repository integrity without checking it.
    git2::opts::strict_hash_verification(false);

    // If we are scanning a large number of Git repositories with a high degree of parallelism
    // (1000 repos with 12 threads, for example), it's common to see all but one thread grind to a
    // halt when enumerating files.
    //
    // It appears that a global LRU cache in the libgit2 C library, used for managing memory-mapped
    // windows, ends up hitting capacity, forcing the threads to sequentialize and thrash.
    //
    // Increasing this libgit2 option ameliorates the problem of maxing out the cache, and the
    // threads can make progress.
    {
        let orig_limit = git2_utils::get_mwindow_mapped_limit();
        let new_limit = (orig_limit * 2).max(16 * 1024 * 1024 * 1024);
        git2_utils::set_mwindow_mapped_limit(new_limit);
        assert_eq!(new_limit, git2_utils::get_mwindow_mapped_limit());
        debug!("git2 mwindow mapped limit increased from {} to {}", orig_limit, new_limit);
    }

    // ---------------------------------------------------------------------------------------------
    // Configure the Rayon global thread pool
    // ---------------------------------------------------------------------------------------------
    debug!("Using {} parallel jobs", args.num_jobs);
    rayon::ThreadPoolBuilder::new()
        .num_threads(args.num_jobs)
        .thread_name(|idx| format!("Scanner {}", idx))
        .build_global()
        .with_context(|| format!("Failed to configure Rayon with {} threads", args.num_jobs))?;

    // ---------------------------------------------------------------------------------------------
    // Open datastore
    // ---------------------------------------------------------------------------------------------
    let mut datastore = Datastore::create_or_open(&args.datastore)?;

    // ---------------------------------------------------------------------------------------------
    // Get temporary directory
    // ---------------------------------------------------------------------------------------------
    let tmpdir = datastore.tmpdir();
    std::fs::create_dir_all(&tmpdir).with_context(|| {
        format!(
            "Failed to create temporary directory {:?} for datastore at {:?}",
            tmpdir, datastore.root_dir()
        )
    })?;

    // ---------------------------------------------------------------------------------------------
    // Load rules
    // ---------------------------------------------------------------------------------------------
    let rules_db = {
        let mut rules = Rules::from_default_rules()
            .context("Failed to load default rules")?;
        if !args.rules.is_empty() {
            let custom_rules = Rules::from_paths(&args.rules)
                .context("Failed to load specified rules files")?;
            rules.extend(custom_rules);
        }
        let rules_db = RulesDatabase::from_rules(rules)
            .context("Failed to compile rules")?;
        rules_db
    };

    // ---------------------------------------------------------------------------------------------
    // Enumerate initial filesystem inputs
    // ---------------------------------------------------------------------------------------------
    let inputs = {
        let mut progress = Progress::new_bytes_spinner("Enumerating inputs", progress_enabled);

        let input_enumerator = {
            let mut ie = FilesystemEnumerator::new(&args.inputs)?;

            // XXX Put a cap on the level of parallelism when enumerating inputs.
            // The libgit2 library has some global state and global locks that result in thrashing
            // and contention when used with too many threads.
            ie.threads(args.num_jobs.min(4));

            ie.max_filesize(args.discovery_args.max_file_size_bytes());

            // Load default ignore file. Note that we have to write it to a file first,
            // because the API for the `ignore` crate doesn't expose something that takes a
            // string.
            let ignore_path = tmpdir.join("default_ignore_rules.conf");
            std::fs::write(&ignore_path, DEFAULT_IGNORE_RULES).with_context(|| {
                format!("Failed to write default ignore rules to {:?}", &ignore_path)
            })?;
            ie.add_ignore(&ignore_path).with_context(|| {
                format!("Failed to load ignore rules from {:?}", &ignore_path)
            })?;

            // Load any specified ignore files
            for ignore_path in args.discovery_args.ignore.iter() {
                debug!("Using ignore rules from {:?}", ignore_path);
                ie.add_ignore(ignore_path).with_context(|| {
                    format!("Failed to load ignore rules from {:?}", ignore_path)
                })?;
            }

            ie
        };

        let inputs = input_enumerator.run(&mut progress)?;
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

    let make_matcher = || -> Result<Matcher> {
        *num_matchers_counter.lock().unwrap() += 1;
        Matcher::new(&rules_db, &seen_blobs, Some(&matcher_stats))
    };

    // a function to convert BlobMatch into regular Match
    let convert_blob_matches =
        |blob: &Blob, matches: Vec<BlobMatch>, provenance: Provenance| -> Vec<Match> {
            assert!(!matches.is_empty());
            let loc_mapping = {
                match matches
                    .iter()
                    .map(|m| m.matching_input_offset_span.end)
                    .max()
                {
                    Some(max_end) => location::LocationMapping::new(&blob.bytes[0..max_end]),
                    None => return Vec::new(),
                }
            };
            matches
                .into_iter()
                .flat_map(|m| Match::new(&loc_mapping, m, &provenance))
                .collect()
        };

    // FIXME: have this print out aggregate rate at finish
    let mut progress = Progress::new_bytes_bar(total_blob_bytes, "Scanning content", progress_enabled);

    // Create a channel pair so that matcher threads can get their results to the database
    // recorder.
    let (send_matches, recv_matches) = mpsc::sync_channel::<Vec<Match>>(512);

    // We create a separate thread for writing matches to the database.
    // The database uses SQLite, which does best with a single writer.
    let match_writer = {
        std::thread::Builder::new().name("Datastore Writer".to_string()).spawn(move || {
            let mut num_matches = 0u64;
            let mut num_added = 0usize;
            // keep reading until all the senders hang up; panic if recording matches fails
            while let Ok(matches) = recv_matches.recv() {
                num_matches += matches.len() as u64;
                num_added += datastore
                    .record_matches(&matches)
                    .expect("should be able to record matches to the database");
            }
            datastore.analyze().expect("should be able to analyze the database");
            (datastore, num_matches, num_added as u64)
        })
        .expect("should be able to start datastore writer thread")
    };

    // ---------------------------------------------------------------------------------------------
    // Scan plain files
    // ---------------------------------------------------------------------------------------------
    inputs.files.par_iter().with_min_len(128).for_each_init(
        || {
            let matcher = make_matcher().expect("should be able to create a matcher");
            (matcher, progress.clone())
        },
        |(matcher, progress), file_result: &FileResult| {
            let fname = &file_result.path;
            let blob = match Blob::from_file(fname) {
                Err(e) => {
                    error!("Failed to load blob from {:?}: {}", fname, e);
                    return;
                }
                Ok(v) => v,
            };
            progress.inc(file_result.num_bytes);
            let provenance = Provenance::FromFile(fname.clone());
            let matches = match matcher.scan_blob(&blob, &provenance) {
                Err(e) => {
                    error!("Failed to scan blob from {:?}: {}", fname, e);
                    return;
                }
                Ok(v) => v,
            };
            if matches.is_empty() {
                return;
            }
            let matches = convert_blob_matches(&blob, matches, provenance);
            send_matches
                .send(matches)
                .expect("should be able to send all matches");
        },
    );

    // ---------------------------------------------------------------------------------------------
    // Scan Git repo inputs
    // ---------------------------------------------------------------------------------------------
    inputs.git_repos.par_iter().for_each(|git_repo_result| {
        git_repo_result
            .blobs
            .par_iter()
            .with_min_len(128)
            .for_each_init(
                || {
                    let repo = open_git_repo(&git_repo_result.path)
                        .ok()
                        .flatten()
                        .expect("should be able to re-open repository");
                    let matcher = make_matcher().expect("should be able to create a matcher");
                    (repo, matcher, progress.clone())
                },
                |(repo, matcher, progress), (oid, size)| {
                    progress.inc(*size);
                    let path = &git_repo_result.path;
                    // debug!("Scanning {} size {} from {:?}", oid, size, path);

                    let blob_id = BlobId::from_oid(oid);

                    // Check for duplicates before even loading the entire blob contents
                    if seen_blobs.contains(&blob_id) {
                        return;
                    }
                    let blob = match repo.find_blob(*oid) {
                        Err(e) => {
                            error!(
                                "Failed to read blob {} from Git repository at {:?}: {}",
                                oid, path, e
                            );
                            return;
                        }
                        Ok(blob) => Blob::new(blob_id, blob.content().to_owned()),
                    };
                    let provenance = Provenance::FromGitRepo(path.to_path_buf());
                    match matcher.scan_blob(&blob, &provenance) {
                        Err(e) => {
                            error!(
                                "Failed to scan blob {} from Git repository at {:?}: {}",
                                oid, path, e
                            );
                            return;
                        }
                        Ok(matches) => {
                            if matches.is_empty() {
                                return;
                            }
                            let matches = convert_blob_matches(&blob, matches, provenance);
                            send_matches
                                .send(matches)
                                .expect("should be able to send all matches");
                        }
                    }
                },
            );
    });

    // ---------------------------------------------------------------------------------------------
    // Wait for all inputs to be scanned and the database thread to finish
    // ---------------------------------------------------------------------------------------------
    // Get rid of the reference to the sending channel after starting the scanners,
    // to ensure things terminate as expected.
    drop(send_matches);
    let (datastore, num_matches, num_new_matches) = match_writer.join().unwrap();
    progress.finish();

    // ---------------------------------------------------------------------------------------------
    // Finalize and report
    // ---------------------------------------------------------------------------------------------
    {
        debug!("{} matchers created during scan", num_matchers_counter.into_inner()?);
        debug!("{} items in the blob ID set", seen_blobs.len());

        let matcher_stats = matcher_stats.into_inner()?;
        let scan_duration = scan_start.elapsed();
        let seen_bytes_per_sec = (matcher_stats.bytes_seen as f64 / scan_duration.as_secs_f64()) as u64;

        println!("Scanned {} from {} blobs in {} ({}/s); {}/{} new matches",
                HumanBytes(matcher_stats.bytes_seen),
                HumanCount(matcher_stats.blobs_seen),
                HumanDuration(scan_duration),
                HumanBytes(seen_bytes_per_sec),
                HumanCount(num_new_matches),
                HumanCount(num_matches),
        );

        if num_matches > 0 {
            let matches_summary = datastore.summarize()?;
            let matches_table = crate::cmd_summarize::summary_table(matches_summary);
            println!();
            matches_table.print_tty(color_enabled)?;
        }

        println!("\nRun the `report` command next to show finding details.");
    }

    Ok(())
}
