use indicatif::{ProgressBar, ProgressStyle};
use std::borrow::Cow;
use std::time::{Duration, Instant};

/// How often should progress bars be redrawn?
pub const PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_millis(500);

// NOTE: indicatif uses an estimation algorithm for ETA and the throughput that doesn't
//       work well for Nosey Parker, resulting in wildly variable and inaccurate values.
//       The problem is with the library's internal `Estimator` type.
//
//       Until that's fixed or we otherwise work around it, we avoid showing ETAs and rates.
//
//       See https://github.com/console-rs/indicatif/issues/394.

// XXX Consider switching from indicatif to status_line: https://docs.rs/status-line/latest/status_line/struct.StatusLine.html

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
    pub fn new_spinner<T: Into<Cow<'static, str>>>(message: T, enabled: bool) -> Self {
        let inner = if enabled {
            let style = ProgressStyle::with_template("{spinner} {msg} [{elapsed_precise}]")
                .expect("progress bar style template should compile");

            let inner = ProgressBar::new_spinner()
                .with_style(style)
                .with_message(message);
            inner.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

            inner
        } else {
            ProgressBar::hidden()
        };

        let finish_style = ProgressStyle::with_template("{msg} [{elapsed_precise}]")
            .expect("progress bar style template should compile");

        Progress {
            inc_since_sync: 0,
            last_sync: Instant::now(),
            inner,
            finish_style: Some(finish_style),
        }
    }

    #[inline]
    pub fn set_message<T: Into<Cow<'static, str>>>(&mut self, message: T) {
        self.inner.set_message(message);
    }

    pub fn new_countup_spinner<T: Into<Cow<'static, str>>>(message: T, enabled: bool) -> Self {
        let inner = if enabled {
            let style =
                ProgressStyle::with_template("{spinner} {msg} {human_len} [{elapsed_precise}]")
                    .expect("progress bar style template should compile");

            let inner = ProgressBar::new_spinner()
                .with_style(style)
                .with_message(message);
            inner.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

            inner
        } else {
            ProgressBar::hidden()
        };

        let finish_style = ProgressStyle::with_template("{msg} [{elapsed_precise}]")
            .expect("progress bar style template should compile");

        Progress {
            inc_since_sync: 0,
            last_sync: Instant::now(),
            inner,
            finish_style: Some(finish_style),
        }
    }

    pub fn new_bytes_spinner<T: Into<Cow<'static, str>>>(message: T, enabled: bool) -> Self {
        let inner = if enabled {
            let style =
                ProgressStyle::with_template("{spinner} {msg} {total_bytes} [{elapsed_precise}]")
                    .expect("progress bar style template should compile");

            let inner = ProgressBar::new_spinner()
                .with_style(style)
                .with_message(message);
            inner.enable_steady_tick(PROGRESS_UPDATE_INTERVAL);

            inner
        } else {
            ProgressBar::hidden()
        };

        let finish_style = ProgressStyle::with_template("{msg} [{elapsed_precise}]")
            .expect("progress bar style template should compile");

        Progress {
            inc_since_sync: 0,
            last_sync: Instant::now(),
            inner,
            finish_style: Some(finish_style),
        }
    }

    pub fn new_bar<T: Into<Cow<'static, str>>>(total: u64, message: T, enabled: bool) -> Self {
        let style = ProgressStyle::with_template(
            "{msg}  {bar} {percent:>3}%  {pos}/{len}  [{elapsed_precise}]",
        )
        .expect("progress bar style template should compile");

        let inner = if enabled {
            let inner = ProgressBar::new(total)
                .with_style(style)
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

    pub fn new_bytes_bar<T: Into<Cow<'static, str>>>(
        total_bytes: u64,
        message: T,
        enabled: bool,
    ) -> Self {
        let style = ProgressStyle::with_template(
            "{msg}  {bar} {percent:>3}%  {bytes}/{total_bytes}  [{elapsed_precise}]",
        )
        .expect("progress bar style template should compile");

        let inner = if enabled {
            let inner = ProgressBar::new(total_bytes)
                .with_style(style)
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
    pub fn inc(&mut self, amount: u64) {
        self.inc_since_sync += amount;
        if self.last_sync.elapsed() >= PROGRESS_UPDATE_INTERVAL {
            self.sync();
        }
    }

    pub fn finish_with_message<T: Into<Cow<'static, str>>>(&mut self, message: T) {
        self.sync();
        match &self.finish_style {
            Some(style) => self.inner.set_style(style.clone()),
            None => {}
        };
        self.inner.finish_with_message(message);
    }

    pub fn finish(&mut self) {
        self.sync();
        match &self.finish_style {
            Some(style) => self.inner.set_style(style.clone()),
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
