use anyhow::{bail, Context, Result};
use indicatif::{HumanBytes, HumanCount, HumanDuration};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, error, error_span, info, trace, warn};

use crate::{args, rule_loader::RuleLoader};

use content_guesser::Guesser;
use input_enumerator::{FilesystemEnumerator, FoundInput};
use progress::Progress;

use noseyparker::blob::{Blob, BlobId};
use noseyparker::blob_id_map::BlobIdMap;
use noseyparker::blob_metadata::BlobMetadata;
use noseyparker::datastore::Datastore;
use noseyparker::defaults::DEFAULT_IGNORE_RULES;
use noseyparker::git_binary::{CloneMode, Git};
use noseyparker::git_url::GitUrl;
use noseyparker::location;
use noseyparker::match_type::Match;
use noseyparker::matcher::{Matcher, ScanResult};
use noseyparker::matcher_stats::MatcherStats;
use noseyparker::provenance::Provenance;
use noseyparker::provenance_set::ProvenanceSet;
use noseyparker::rules_database::RulesDatabase;

// -------------------------------------------------------------------------------------------------
/// Something that can be turned into a parallel iterator of blobs
trait ParallelBlobIterator {
    type Iter: ParallelIterator<Item = Result<(ProvenanceSet, Blob)>>;

    fn into_blob_iter(self) -> Result<Option<Self::Iter>>;
}

// -------------------------------------------------------------------------------------------------
/// Blob content from an extensible enumerator
#[derive(serde::Deserialize)]
pub enum Content {
    #[serde(rename = "content_base64")]
    Base64(#[serde(with = "bstring_serde::BStringBase64")] bstr::BString),

    #[serde(rename = "content")]
    Utf8(String),
}

impl Content {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Content::Base64(s) => s.as_slice(),
            Content::Utf8(s) => s.as_bytes(),
        }
    }
}

// -------------------------------------------------------------------------------------------------
/// An entry deserialized from an extensible enumerator
#[derive(serde::Deserialize)]
struct EnumeratorBlobResult {
    #[serde(flatten)]
    pub content: Content,

    pub provenance: serde_json::Value,
}

// -------------------------------------------------------------------------------------------------
/// A parallel iterator for an `input_enumerator::EnumeratorFileResult`.
struct EnumeratorFileIter {
    inner: input_enumerator::EnumeratorFileResult,
    reader: std::io::BufReader<std::fs::File>,
}

impl ParallelBlobIterator for input_enumerator::EnumeratorFileResult {
    type Iter = EnumeratorFileIter;

    fn into_blob_iter(self) -> Result<Option<Self::Iter>> {
        let file = std::fs::File::open(&self.path)?;
        let reader = std::io::BufReader::new(file);
        Ok(Some(EnumeratorFileIter {
            inner: self,
            reader,
        }))
    }
}

// Enumerator file parallelism approach:
//
// - Split into lines sequentially
// - Parallelize JSON deserialization (JSON is an expensive serialization format, but easy to sling
//   around, hence used here -- another format like Arrow or msgpack would be much more efficient)
impl ParallelIterator for EnumeratorFileIter {
    type Item = Result<(ProvenanceSet, Blob)>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        use std::io::BufRead;
        (1usize..)
            .zip(self.reader.lines())
            .filter_map(|(line_num, line)| line.map(|line| (line_num, line)).ok())
            .par_bridge()
            .map(|(line_num, line)| {
                let e: EnumeratorBlobResult = serde_json::from_str(&line).with_context(|| {
                    format!("Error in enumerator {}:{line_num}", self.inner.path.display())
                })?;
                let provenance = Provenance::from_extended(e.provenance).into();
                let blob = Blob::from_bytes(e.content.as_bytes().to_owned());
                Ok((provenance, blob))
            })
            .drive_unindexed(consumer)
    }
}

// --------------------------------------------------------------------------------
/// A parallel iterator for in `input_enumerator::FileResult`
struct FileResultIter {
    inner: input_enumerator::FileResult,
    blob: Blob,
}

impl ParallelBlobIterator for input_enumerator::FileResult {
    type Iter = FileResultIter;

    fn into_blob_iter(self) -> Result<Option<Self::Iter>> {
        let blob = Blob::from_file(&self.path)
            .with_context(|| format!("Failed to load blob from {}", self.path.display()))?;
        Ok(Some(FileResultIter { inner: self, blob }))
    }
}

impl ParallelIterator for FileResultIter {
    type Item = Result<(ProvenanceSet, Blob)>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        use rayon::iter::plumbing::Folder;

        let item = Ok((Provenance::from_file(self.inner.path).into(), self.blob));
        consumer.into_folder().consume(item).complete()
    }
}

// --------------------------------------------------------------------------------
/// A parallel iterator for an `input_enumerator::GitRepoResult`
struct GitRepoResultIter {
    inner: input_enumerator::GitRepoResult,
}

impl ParallelBlobIterator for input_enumerator::GitRepoResult {
    type Iter = GitRepoResultIter;

    fn into_blob_iter(self) -> Result<Option<Self::Iter>> {
        Ok(Some(GitRepoResultIter { inner: self }))
    }
}

impl ParallelIterator for GitRepoResultIter {
    type Item = Result<(ProvenanceSet, Blob)>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        let repo = self.inner.repository.into_sync();
        let repo_path = Arc::new(self.inner.path.clone());
        self.inner
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
            .with_min_len(1024)
            .map_init(
                || repo.to_thread_local(),
                |repo, md| -> Result<(ProvenanceSet, Blob)> {
                    let blob_id = md.blob_oid;

                    let blob = || -> Result<Blob> {
                        let mut blob = repo.find_object(blob_id)?.try_into_blob()?;
                        let data = std::mem::take(&mut blob.data); // avoid a copy
                        Ok(Blob::new(BlobId::from(&blob_id), data))
                    }()
                    .with_context(|| {
                        format!(
                            "Failed to read blob {blob_id} from Git repository at {}",
                            repo_path.display(),
                        )
                    })?;

                    let provenance =
                        ProvenanceSet::try_from_iter(md.first_seen.into_iter().map(|e| {
                            Provenance::from_git_repo_with_first_commit(
                                repo_path.clone(),
                                e.commit_metadata,
                                e.path,
                            )
                        }))
                        .unwrap_or_else(|| Provenance::from_git_repo(repo_path.clone()).into());

                    Ok((provenance, blob))
                },
            )
            .drive_unindexed(consumer)
    }
}

// -------------------------------------------------------------------------------------------------
struct EnumeratorConfig {
    enumerate_git_history: bool,
    collect_git_metadata: bool,
    gitignore: input_enumerator::Gitignore,
}

// --------------------------------------------------------------------------------
enum FoundInputIter {
    File(FileResultIter),
    GitRepo(GitRepoResultIter),
    EnumeratorFile(EnumeratorFileIter),
}

impl ParallelBlobIterator for (&EnumeratorConfig, FoundInput) {
    type Iter = FoundInputIter;

    fn into_blob_iter(self) -> Result<Option<Self::Iter>> {
        let (cfg, input) = self;
        match input {
            FoundInput::File(i) => Ok(i.into_blob_iter()?.map(FoundInputIter::File)),

            FoundInput::Directory(i) => {
                let path = &i.path;
                if cfg.enumerate_git_history {
                    match input_enumerator::open_git_repo(path)? {
                        Some(repository) => {
                            let t1 = Instant::now();
                            debug!("Found Git repository at {}", path.display());

                            let result = if cfg.collect_git_metadata {
                                input_enumerator::GitRepoWithMetadataEnumerator::new(
                                    path,
                                    repository,
                                    &cfg.gitignore,
                                )
                                .run()?
                            } else {
                                input_enumerator::GitRepoEnumerator::new(path, repository).run()?
                            };

                            debug!(
                                "Enumerated Git repository at {} in {:.6}s",
                                path.display(),
                                t1.elapsed().as_secs_f64()
                            );

                            result
                                .into_blob_iter()
                                .map(|i| i.map(FoundInputIter::GitRepo))
                        }
                        None => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }

            FoundInput::EnumeratorFile(i) => {
                Ok(i.into_blob_iter()?.map(FoundInputIter::EnumeratorFile))
            }
        }
    }
}

impl ParallelIterator for FoundInputIter {
    type Item = Result<(ProvenanceSet, Blob)>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        match self {
            FoundInputIter::File(i) => i.drive_unindexed(consumer),
            FoundInputIter::GitRepo(i) => i.drive_unindexed(consumer),
            FoundInputIter::EnumeratorFile(i) => i.drive_unindexed(consumer),
        }
    }
}

// --------------------------------------------------------------------------------

/// This command scans multiple filesystem inputs for secrets.
/// The implementation enumerates content in parallel, scans the enumerated content in parallel,
/// and records found matches to a SQLite database sequentially.
pub fn run(global_args: &args::GlobalArgs, args: &args::ScanArgs) -> Result<()> {
    // ---------------------------------------------------------------------------------------------
    // Parse args
    // ---------------------------------------------------------------------------------------------
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
    init_progress.set_message("Initializing (thread pools)...");
    rayon::ThreadPoolBuilder::new()
        .num_threads(args.num_jobs)
        .thread_name(|idx| format!("scanner-{idx}"))
        .build_global()
        .context("Failed to initialize Rayon")?;

    // ---------------------------------------------------------------------------------------------
    // Open datastore
    // ---------------------------------------------------------------------------------------------
    init_progress.set_message("Initializing (datastore)...");
    let mut datastore =
        Datastore::create_or_open(&args.datastore, global_args.advanced.sqlite_cache_size)
            .with_context(|| {
                format!("Failed to open datastore at {}", &args.datastore.display())
            })?;

    // ---------------------------------------------------------------------------------------------
    // Load rules and record them to the datastore
    // ---------------------------------------------------------------------------------------------
    init_progress.set_message("Initializing (rules)...");
    let rules_db = {
        let loaded = RuleLoader::from_rule_specifiers(&args.rules)
            .load()
            .context("Failed to load rules")?;
        let resolved = loaded
            .resolve_enabled_rules()
            .context("Failed to resolve rules")?;
        let rules_db = RulesDatabase::from_rules(resolved.into_iter().cloned().collect())
            .context("Failed to compile rules")?;

        || -> Result<()> {
            let tx = datastore.begin()?;
            tx.record_rules(rules_db.rules())?;
            tx.commit()
        }()
        .context("Failed to record rules to the datastore")?;

        rules_db
    };
    drop(init_progress);

    // ---------------------------------------------------------------------------------------------
    // Gather list of all git repos to clone or update
    // ---------------------------------------------------------------------------------------------
    let repo_urls = {
        let mut repo_urls = args.input_specifier_args.git_url.clone();
        repo_urls.extend(enumerate_github_repos(global_args, args)?);
        repo_urls.sort();
        repo_urls.dedup();
        repo_urls
    };

    // ---------------------------------------------------------------------------------------------
    // Clone or update all mentioned Git URLs; gather set of input roots for scanning
    // ---------------------------------------------------------------------------------------------
    let input_roots = {
        let mut input_roots = args.input_specifier_args.path_inputs.clone();
        if !repo_urls.is_empty() {
            input_roots.extend(clone_git_repo_urls(global_args, args, &datastore, repo_urls)?);
        }
        input_roots.sort();
        input_roots.dedup();
        input_roots
    };

    if input_roots.is_empty() && args.input_specifier_args.enumerators.is_empty() {
        bail!("No inputs to scan");
    }

    // we'll need this later
    let blobs_dir = datastore.blobs_dir();

    // ---------------------------------------------------------------------------------------------
    // Kick off input enumeration in a separate thread, writing results to a channel
    // ---------------------------------------------------------------------------------------------
    let scan_start = Instant::now();
    let (enum_thread, input_recv, gitignore) = {
        let (fs_enumerator, gitignore) = make_fs_enumerator(args, &datastore, input_roots)
            .context("Failed to initialize filesystem enumerator")?;

        // Create a pair of channels for the input enumeration
        let channel_size = std::cmp::max(args.num_jobs * 32, 256);
        let (input_send, input_recv) = crossbeam_channel::bounded(channel_size);

        let enumerators = args.input_specifier_args.enumerators.clone();

        let input_enumerator_thread = std::thread::Builder::new()
            .name("input_enumerator".to_string())
            .spawn(move || -> Result<_> {
                // Inject input enumerator files; to be enumerated downstream
                for path in enumerators {
                    let ef = input_enumerator::EnumeratorFileResult { path };
                    input_send.send(FoundInput::EnumeratorFile(ef))?;
                }

                // Find inputs from disk. This is parallelized internally in the `.run()` method.
                if let Some(fs_enumerator) = fs_enumerator {
                    fs_enumerator.run(input_send.clone())?;
                }

                Ok(())
            })
            .context("Failed to enumerate filesystem inputs")?;

        (input_enumerator_thread, input_recv, gitignore)
    };

    // ---------------------------------------------------------------------------------------------
    // Kick off datastore persistence in a separate thread, providing a channel for scanners to
    // write into. (SQLite works best with a single writer)
    // ---------------------------------------------------------------------------------------------
    let (datastore_thread, send_ds) = {
        let channel_size = std::cmp::max(args.num_jobs, 64) * DATASTORE_BATCH_SIZE;
        let (send_ds, recv_ds) = crossbeam_channel::bounded::<DatastoreMessage>(channel_size);

        let datastore_thread = std::thread::Builder::new()
            .name("datastore".to_string())
            .spawn(move || datastore_writer(datastore, recv_ds))?;

        (datastore_thread, send_ds)
    };

    // ---------------------------------------------------------------------------------------------
    // Scan enumerated inputs, sending results to the datastore thread
    //
    // Don't check the overall result until after checking the other threads,
    // in order to give more comprehensible error reporting when something goes wrong.
    // ---------------------------------------------------------------------------------------------
    let mut progress = Progress::new_bytes_spinner("Scanning content", progress_enabled);

    let enum_cfg = EnumeratorConfig {
        enumerate_git_history: match args.input_specifier_args.git_history {
            args::GitHistoryMode::Full => true,
            args::GitHistoryMode::None => false,
        },
        collect_git_metadata: match args.metadata_args.git_blob_provenance {
            args::GitBlobProvenanceMode::FirstSeen => true,
            args::GitBlobProvenanceMode::Minimal => false,
        },
        gitignore,
    };

    let t1 = Instant::now();
    let num_blob_processors = Mutex::new(0u64); // how many blob processors have been initialized?
    let matcher_stats = Mutex::new(MatcherStats::default());
    let seen_blobs = BlobIdMap::new();
    let matcher = Matcher::new(&rules_db, &seen_blobs, Some(&matcher_stats))?;

    let blob_copier = match args.copy_blobs {
        args::CopyBlobsMode::All | args::CopyBlobsMode::Matching => match args.copy_blobs_format {
            #[cfg(feature = "parquet")]
            args::CopyBlobsFormat::Parquet => {
                BlobCopier::Parquet(ParquetBlobCopier::new(blobs_dir, args.num_jobs)?)
            }
            args::CopyBlobsFormat::Files => BlobCopier::Files(FilesBlobCopier::new(blobs_dir)),
        },
        args::CopyBlobsMode::None => BlobCopier::Noop,
    };

    let blob_processor_init_time = Mutex::new(t1.elapsed());

    let make_blob_processor = || -> BlobProcessor {
        let t1 = Instant::now();
        let matcher = matcher.clone();
        *num_blob_processors.lock().unwrap() += 1;
        let guesser = Guesser::new().expect("should be able to create filetype guessser");
        let proc = BlobProcessor {
            matcher,
            guesser,
            snippet_length: args.snippet_length,
            blob_metadata_recording_mode: args.metadata_args.blob_metadata,
            blob_copier: blob_copier.clone(),
            copy_blobs_mode: args.copy_blobs,
        };
        *blob_processor_init_time.lock().unwrap() += t1.elapsed();

        proc
    };

    let scan_res: Result<()> = input_recv
        .into_iter()
        .par_bridge()
        .filter_map(|input: FoundInput| match (&enum_cfg, input).into_blob_iter() {
            Err(e) => {
                error!("Error enumerating input: {e:#}");
                None
            }
            Ok(blob_iter) => blob_iter,
        })
        .flatten()
        .try_for_each_init(
            || (make_blob_processor(), progress.clone()),
            move |(processor, progress), entry| {
                let (provenance, blob) = match entry {
                    Err(e) => {
                        error!("Error loading input: {e:#}");
                        return Ok(());
                    }
                    Ok(entry) => entry,
                };

                progress.inc(blob.len().try_into().unwrap());
                match processor.run(provenance, blob) {
                    Err(e) => {
                        error!("Error scanning input: {e:#}");
                    }
                    Ok(None) => {
                        // nothing to record
                    }
                    Ok(Some(msg)) => {
                        send_ds.send(msg)?;
                    }
                }
                Ok(())
            },
        );

    // ---------------------------------------------------------------------------------------------
    // Wait for all inputs to be enumerated and scanned and the database thread to finish
    // ---------------------------------------------------------------------------------------------
    enum_thread
        .join()
        .unwrap()
        .context("Failed to enumerate inputs")?;

    let (mut datastore, num_matches, num_new_matches) = datastore_thread
        .join()
        .unwrap()
        .context("Failed to save results to the datastore")?;

    blob_copier.close()?;

    // now finally check the result of the scanners
    scan_res.context("Failed to scan inputs")?;

    progress.finish();

    datastore.check_match_redundancies()?;

    // ---------------------------------------------------------------------------------------------
    // Finalize and report
    // ---------------------------------------------------------------------------------------------
    {
        debug!(
            "{} blob processors created in {:.3}s during scan",
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
            let table = crate::cmd_summarize::summary_table(&summary, /* simple= */ true);
            println!();
            table.print_tty(global_args.use_color(std::io::stdout()))?;
        }

        println!("\nRun the `report` command next to show finding details.");
    }

    Ok(())
}

#[derive(Clone)]
enum BlobCopier {
    Noop,
    Files(FilesBlobCopier),
    #[cfg(feature = "parquet")]
    Parquet(ParquetBlobCopier),
}

impl BlobCopier {
    fn copy(&self, blob: &Blob) -> Result<()> {
        match self {
            BlobCopier::Noop => Ok(()),
            BlobCopier::Files(c) => c.copy(blob),
            #[cfg(feature = "parquet")]
            BlobCopier::Parquet(c) => c.copy(blob),
        }
    }

    fn close(self) -> Result<()> {
        match self {
            BlobCopier::Noop | BlobCopier::Files(_) => Ok(()),
            #[cfg(feature = "parquet")]
            BlobCopier::Parquet(c) => c.close(),
        }
    }
}

#[derive(Clone)]
struct FilesBlobCopier {
    blobs_dir: PathBuf,
}

impl FilesBlobCopier {
    fn new(blobs_dir: PathBuf) -> Self {
        Self { blobs_dir }
    }
}

impl FilesBlobCopier {
    fn copy(&self, blob: &Blob) -> Result<()> {
        let blob_id = blob.id.hex();
        let output_dir = self.blobs_dir.join(&blob_id[..2]);
        let output_path = output_dir.join(&blob_id[2..]);
        trace!("saving blob to {}", output_path.display());
        match std::fs::create_dir(&output_dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                bail!("Failed to create blob directory at {}: {e}", output_dir.display(),);
            }
        }
        std::fs::write(&output_path, &blob.bytes).with_context(|| {
            format!("Failed to write blob contents to {}", output_path.display())
        })?;

        Ok(())
    }
}

#[cfg(feature = "parquet")]
#[derive(Clone)]
struct ParquetBlobCopier {
    writer_pool: Arc<object_pool::Pool<parquet::arrow::arrow_writer::ArrowWriter<std::fs::File>>>,
    field_blob_id: Arc<arrow_schema::Field>,
    field_content: Arc<arrow_schema::Field>,
    field_content_len: Arc<arrow_schema::Field>,
}

#[cfg(feature = "parquet")]
impl ParquetBlobCopier {
    fn new(blobs_dir: PathBuf, num_writers: usize) -> Result<Self> {
        use arrow_schema::{DataType, Field};

        let field_blob_id = Field::new("blob_id", DataType::Utf8, /* nullable= */ false);
        let field_content = Field::new("content", DataType::Binary, /* nullable= */ false);
        let field_content_len =
            Field::new("content_len", DataType::UInt64, /* nullable= */ false);

        let writer_pool = {
            use arrow_schema::Schema;
            use parquet::arrow::arrow_writer::ArrowWriter;
            use parquet::file::properties::WriterProperties;
            use std::fs::File;

            let mut writers = Vec::with_capacity(num_writers);

            let schema = Arc::new(Schema::new(vec![
                field_blob_id.clone(),
                field_content.clone(),
                field_content_len.clone(),
            ]));
            let props = Some(
                WriterProperties::builder()
                    .set_compression(parquet::basic::Compression::ZSTD(Default::default()))
                    // .set_max_row_group_size(128 * 1024)
                    .build(),
            );

            // choose parquet filenames to avoid clobbering existing files
            let num_existing_files =
                glob::glob(&format!("{}/blobs.*.parquet", blobs_dir.display()))?.count();
            for i in num_existing_files..num_writers + num_existing_files {
                let outfile = blobs_dir.join(format!("blobs.{i:02}.parquet"));
                let outfile = File::create(outfile)?;
                let writer = ArrowWriter::try_new(outfile, schema.clone(), props.clone())?;
                writers.push(writer);
            }
            Arc::new(object_pool::Pool::from_vec(writers))
        };

        Ok(Self {
            writer_pool,
            field_blob_id: Arc::new(field_blob_id),
            field_content: Arc::new(field_content),
            field_content_len: Arc::new(field_content_len),
        })
    }

    fn copy(&self, blob: &Blob) -> Result<()> {
        use arrow_array::{
            ArrayRef, BinaryArray, RecordBatch, StringArray, StructArray, UInt64Array,
        };

        let mut writer = self
            .writer_pool
            .try_pull()
            .expect("should be able to get a parquet writer");

        let blob_ids = Arc::new(StringArray::from(vec![blob.id.hex()]));
        let contents = Arc::new(BinaryArray::from(vec![blob.bytes.as_slice()]));
        let content_lens = Arc::new(UInt64Array::from(vec![blob.bytes.len() as u64]));

        let batch = RecordBatch::from(StructArray::from(vec![
            (self.field_blob_id.clone(), blob_ids as ArrayRef),
            (self.field_content.clone(), contents as ArrayRef),
            (self.field_content_len.clone(), content_lens as ArrayRef),
        ]));
        writer.write(&batch)?;

        let writer_size_bytes = writer.memory_size();
        if writer_size_bytes >= 128 * 1024 * 1024 {
            let num_in_progress = writer.in_progress_rows();
            let t1 = Instant::now();
            writer.flush()?;
            let t1e = t1.elapsed();
            trace!(
                "Writer size is {num_in_progress} rows / {:.1} MiB; flushed in {:.3}s",
                writer_size_bytes as f64 / 1024.0 / 1024.0,
                t1e.as_secs_f64()
            );
        }

        Ok(())
    }

    fn close(self) -> Result<()> {
        while let Some(writer) = self.writer_pool.try_pull() {
            let (_writer_pool, writer) = writer.detach();
            writer.close()?;
        }
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
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

// -------------------------------------------------------------------------------------------------
/// A combined matcher, content type guesser, and a number of parameters that don't change within
/// one `scan` run
struct BlobProcessor<'a> {
    matcher: Matcher<'a>,
    guesser: Guesser,

    snippet_length: usize,
    blob_metadata_recording_mode: args::BlobMetadataMode,
    copy_blobs_mode: args::CopyBlobsMode,
    blob_copier: BlobCopier,
}

impl<'a> BlobProcessor<'a> {
    fn run(&mut self, provenance: ProvenanceSet, blob: Blob) -> Result<Option<DatastoreMessage>> {
        let blob_id = blob.id.hex();
        let _span = error_span!("matcher", blob_id, bytes = blob.len()).entered();

        let (res, scan_us, scan_mbps) = if tracing::enabled!(tracing::Level::TRACE) {
            let t1 = Instant::now();
            let res = self.matcher.scan_blob(&blob, &provenance)?;
            let t1e = t1.elapsed();
            (res, t1e.as_micros(), blob.len() as f64 / 1024.0 / 1024.0 / t1e.as_secs_f64())
        } else {
            let res = self.matcher.scan_blob(&blob, &provenance)?;
            (res, Default::default(), Default::default())
        };

        match res {
            // blob already seen, but with no matches; nothing to do!
            ScanResult::SeenSansMatches => {
                trace!(us = scan_us, mbps = scan_mbps, status = "seen_nomatch");
                Ok(None)
            }

            // blob already seen; all we need to do is record its provenance
            ScanResult::SeenWithMatches => {
                trace!(us = scan_us, mbps = scan_mbps, status = "seen_match");
                let metadata = BlobMetadata {
                    id: blob.id,
                    num_bytes: blob.len(),
                    mime_essence: None,
                    charset: None,
                };
                Ok(Some((provenance, metadata, Vec::new())))
            }

            // blob has not been seen; need to record blob metadata, provenance, and matches
            ScanResult::New(matches) => {
                trace!(us = scan_us, mbps = scan_mbps, status = "new", matches = matches.len());

                let do_copy = match self.copy_blobs_mode {
                    args::CopyBlobsMode::All => true,
                    args::CopyBlobsMode::Matching => !matches.is_empty(),
                    args::CopyBlobsMode::None => false,
                };
                if do_copy {
                    self.blob_copier
                        .copy(&blob)
                        .context("Failed to copy blob")?;
                }

                // If there are no matches, we can bail out here and avoid recording anything.
                // UNLESS the `--blob-metadata=all` mode was specified; then we need to record the
                // provenance for _all_ seen blobs.
                if self.blob_metadata_recording_mode != args::BlobMetadataMode::All
                    && matches.is_empty()
                {
                    return Ok(None);
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

                Ok(Some((provenance, metadata, matches)))
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
/// Initialize a `FilesystemEnumerator` based on the command-line arguments and datastore.
/// Also initialize a `Gitignore` that is the same as that used by the filesystem enumerator.
fn make_fs_enumerator(
    args: &args::ScanArgs,
    datastore: &Datastore,
    input_roots: Vec<PathBuf>,
) -> Result<(Option<FilesystemEnumerator>, input_enumerator::Gitignore)> {
    // FIXME: eliminate this code duplication: logic repeated 2x in input-enumerator
    let mut gitignore_builder = input_enumerator::GitignoreBuilder::new("");

    // Load default ignore file. Note that we have to write it to a file first,
    // because the API for the `ignore` crate doesn't expose something that takes a
    // string.
    let ignore_path = datastore.scratch_dir().join("default_ignore_rules.conf");
    std::fs::write(&ignore_path, DEFAULT_IGNORE_RULES).with_context(|| {
        format!("Failed to write default ignore rules to {}", ignore_path.display())
    })?;

    // Load any specified ignore files
    let ipaths = std::iter::once(&ignore_path).chain(args.content_filtering_args.ignore.iter());
    for ignore_path in ipaths {
        if let Some(e) = gitignore_builder.add(ignore_path) {
            return Err(e).with_context(|| {
                format!("Failed to load ignore rules from {}", ignore_path.display())
            });
        }
    }

    let gitignore = gitignore_builder.build()?;

    if input_roots.is_empty() {
        Ok((None, gitignore))
    } else {
        let mut ie = FilesystemEnumerator::new(&input_roots)?;

        ie.threads(args.num_jobs);
        ie.max_filesize(args.content_filtering_args.max_file_size_bytes());
        if args.input_specifier_args.git_history == args::GitHistoryMode::None {
            ie.enumerate_git_history(false);
        }

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

        // Determine whether to collect git metadata or not
        let collect_git_metadata = match args.metadata_args.git_blob_provenance {
            args::GitBlobProvenanceMode::FirstSeen => true,
            args::GitBlobProvenanceMode::Minimal => false,
        };
        ie.collect_git_metadata(collect_git_metadata);

        Ok((Some(ie), gitignore))
    }
}

// -------------------------------------------------------------------------------------------------
/// Enumerate mentioned GitHub repositories via the GitHub REST API, returning vector of repo urls
#[cfg(feature = "github")]
fn enumerate_github_repos(
    global_args: &args::GlobalArgs,
    args: &args::ScanArgs,
) -> Result<Vec<GitUrl>> {
    let mut repo_urls = vec![];

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
            global_args.use_progress(),
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

    Ok(repo_urls)
}

/// Enumerate mentioned GitHub repositories via the GitHub REST API, returning vector of repo urls
#[cfg(not(feature = "github"))]
fn enumerate_github_repos(
    _global_args: &args::GlobalArgs,
    _args: &args::ScanArgs,
) -> Result<Vec<GitUrl>> {
    Ok(vec![])
}

// -------------------------------------------------------------------------------------------------
type DatastoreMessage = (ProvenanceSet, BlobMetadata, Vec<(Option<f64>, Match)>);

// XXX: expose the following as CLI parameters?
const DATASTORE_BATCH_SIZE: usize = 16 * 1024;
const DATASTORE_COMMIT_INTERVAL: Duration = Duration::from_secs(1);

// -------------------------------------------------------------------------------------------------
/// Read messages from a channel, and write them into the datastore.
///
/// Big idea: read until all the senders hang up; panic if recording matches fails.
///
/// Record all messages chunked transactions, trying to commit at least every
/// `DATASTORE_COMMIT_INTERVAL`.
fn datastore_writer(
    mut datastore: Datastore,
    recv_ds: crossbeam_channel::Receiver<DatastoreMessage>,
) -> Result<(Datastore, u64, u64)> {
    let _span = error_span!("datastore", "{}", datastore.root_dir().display()).entered();
    let mut total_recording_time: std::time::Duration = Default::default();

    let mut num_matches_added: u64 = 0;
    let mut total_messages: u64 = 0;

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

    // record any remaining messages
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
}

// -------------------------------------------------------------------------------------------------
/// Clone the repos given in `repo_urls` inside of the datastore's clones directory.
fn clone_git_repo_urls(
    global_args: &args::GlobalArgs,
    args: &args::ScanArgs,
    datastore: &Datastore,
    repo_urls: Vec<GitUrl>,
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::with_capacity(repo_urls.len());

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
        Progress::new_bar(repo_urls.len() as u64, "Fetching Git repos", global_args.use_progress());

    let cloning_repos = Mutex::new(vec![]);

    for repo_url in repo_urls {
        {
            cloning_repos.lock().unwrap().push(repo_url.clone());
        }
        progress.set_message(format!("Fetching Git repos ({repo_url})"));

        let output_dir = match datastore.clone_destination(&repo_url) {
            Err(e) => {
                progress.suspend(|| {
                    error!(
                        "Failed to determine output directory for {repo_url}: {e}; skipping scan"
                    );
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
                    paths.push(output_dir);
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
        paths.push(output_dir);
        progress.inc(1);
    }

    progress.finish_with_message("Fetching Git repos");
    Ok(paths)
}
