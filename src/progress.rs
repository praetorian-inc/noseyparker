use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use std::borrow::Cow;
use std::time::{Duration, Instant};

/// How often should progress bars be redrawn?
pub const PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_millis(500);

lazy_static! {
    static ref INDEFINITE_BYTES_STYLE: ProgressStyle =
        ProgressStyle::with_template("{spinner} {msg} {total_bytes} [{elapsed_precise}]")
            .expect("progress bar style template should compile");

    static ref INDEFINITE_BYTES_FINISH_STYLE: ProgressStyle =
        ProgressStyle::with_template("{msg} [{elapsed_precise}]")
            .expect("progress bar style template should compile");

    // NOTE: indicatif uses an estimation algorithm for ETA and the throughput that doesn't
    //       work well for this use case, resulting in wildly variable and inaccurate values.
    //       The problem is with the library's internal `Estimator` type.
    //
    //       Until that's fixed or we otherwise work around it, we avoid showing ETAs and rates.
    //
    //       See https://github.com/console-rs/indicatif/issues/394.

    static ref DEFINITE_BYTES_STYLE: ProgressStyle =
        ProgressStyle::with_template("{msg}  {bar} {percent:>3}%  {bytes}/{total_bytes}  [{elapsed_precise}]")
            .expect("progress bar style template should compile");

    static ref DEFINITE_UNITLESS_STYLE: ProgressStyle =
        ProgressStyle::with_template("{msg}  {bar} {percent:>3}%  {pos}/{len}  [{elapsed_precise}]")
            .expect("progress bar style template should compile");
}


/// Wraps an `indicatif::ProgressBar` with a local buffer to reduce update contention overhead.
/// Updates are batched an the progress bar is updated only every `PROGRESS_UPDATE_INTERVAL`.
///
/// XXX this abstraction is probably a little bit broken: if there are multiple clones of a
/// progress bar out there, and `.finish*()` is called on one of them, the others may have
/// unsynced local counts.
pub struct Progress {
    inc_since_sync: u64,
    last_sync: Instant,
    inner: ProgressBar,
    finish_style: Option<ProgressStyle>,
}

impl Progress {
    pub fn new_bytes_spinner<T: Into<Cow<'static, str>>>(message: T, enabled: bool) -> Self {
        let inner = if enabled {
            let inner = ProgressBar::new_spinner()
                .with_style(INDEFINITE_BYTES_STYLE.clone())
                .with_message(message);

            inner.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

            inner
        } else {
            ProgressBar::hidden()
        };

        Progress {
            inc_since_sync: 0,
            last_sync: Instant::now(),
            inner,
            finish_style: Some(INDEFINITE_BYTES_FINISH_STYLE.clone()),
        }
    }

    pub fn new_bar<T: Into<Cow<'static, str>>>(total: u64, message: T, enabled: bool) -> Self {
        let inner = if enabled {
            let inner = ProgressBar::new(total)
                .with_style(DEFINITE_UNITLESS_STYLE.clone())
                .with_message(message);

            inner.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

            inner
        } else {
            ProgressBar::hidden()
        };

        Progress {
            inc_since_sync: 0,
            last_sync: Instant::now(),
            inner,
            finish_style: None,
        }
    }

    pub fn new_bytes_bar<T: Into<Cow<'static, str>>>(total_bytes: u64, message: T, enabled: bool) -> Self {
        let inner = if enabled {
            let inner = ProgressBar::new(total_bytes)
                .with_style(DEFINITE_BYTES_STYLE.clone())
                .with_message(message);

            inner.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

            inner
        } else {
            ProgressBar::hidden()
        };

        Progress {
            inc_since_sync: 0,
            last_sync: Instant::now(),
            inner,
            finish_style: None,
        }
    }

    #[inline]
    pub fn suspend<F: FnOnce() -> R, R>(&self, f: F) -> R {
        self.inner.suspend(f)
    }

    #[inline]
    pub fn inc(&mut self, bytes_seen: u64) {
        self.inc_since_sync += bytes_seen;
        if self.last_sync.elapsed() >= PROGRESS_UPDATE_INTERVAL {
            self.sync();
        }
    }

    pub fn finish_with_message<T: Into<Cow<'static, str>>>(&mut self, message: T) {
        self.sync();
        match &self.finish_style {
            Some(style) => { self.inner.set_style(style.clone()) }
            None => {}
        };
        self.inner.finish_with_message(message);
    }

    pub fn finish(&mut self) {
        self.sync();
        match &self.finish_style {
            Some(style) => { self.inner.set_style(style.clone()) }
            None => {}
        };
        self.inner.finish();
    }

    fn sync(&mut self) {
        self.inner.inc(self.inc_since_sync);
        self.inc_since_sync = 0;
        self.last_sync = Instant::now();
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.sync();
    }
}

impl Clone for Progress {
    fn clone(&self) -> Self {
        Progress {
            inc_since_sync: 0,
            last_sync: Instant::now(),
            inner: self.inner.clone(),
            finish_style: self.finish_style.clone(),
        }
    }
}
