use std::time::{Duration, Instant};

// -------------------------------------------------------------------------------------------------
// RuleProfile
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct RuleProfile {
    raw_match_counts: Vec<u64>,
    stage2_durations: Vec<Duration>,
}

impl RuleProfile {
    /// Update this rule profile by combining it with the contents of another one.
    pub fn update(&mut self, other: &Self) {
        if other.raw_match_counts.len() >= self.raw_match_counts.len() {
            self.raw_match_counts
                .resize(other.raw_match_counts.len(), 0);
        }
        for (i, c) in other.raw_match_counts.iter().enumerate() {
            self.raw_match_counts[i] += c;
        }

        if other.stage2_durations.len() >= self.stage2_durations.len() {
            self.stage2_durations
                .resize(other.stage2_durations.len(), Duration::default());
        }
        for (i, c) in other.stage2_durations.iter().enumerate() {
            self.stage2_durations[i] += *c;
        }
    }

    fn resize_to_fit(&mut self, rule_id: usize) {
        let cap = rule_id + 1;
        if cap > self.raw_match_counts.len() {
            self.raw_match_counts.resize(cap, Default::default());
            self.stage2_durations.resize(cap, Default::default());
        }
    }

    #[inline]
    pub fn increment_match_count(&mut self, rule_id: usize, count: u64) {
        self.resize_to_fit(rule_id);
        self.raw_match_counts[rule_id] += count;
    }

    #[inline]
    pub fn increment_stage2_duration(&mut self, rule_id: usize, duration: Duration) {
        self.resize_to_fit(rule_id);
        self.stage2_durations[rule_id] += duration;
    }

    pub fn get_entries(&self) -> Vec<RuleProfileEntry> {
        self.raw_match_counts
            .iter()
            .cloned()
            .zip(self.stage2_durations.iter().cloned())
            .enumerate()
            .map(|(i, (c, d))| RuleProfileEntry {
                rule_id: i,
                raw_match_count: c,
                stage2_duration: d,
            })
            .collect()
    }

    #[inline]
    pub fn time_stage2(&mut self, rule_id: usize) -> RuleStage2Timer<'_> {
        RuleStage2Timer::new(self, rule_id)
    }
}

// -------------------------------------------------------------------------------------------------
// RuleProfileEntry
// -------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct RuleProfileEntry {
    /// The rule ID this entry corresponds to
    pub rule_id: usize,

    /// How many raw matches for this rule were produced by the first stage of matching using
    /// Vectorscan?
    pub raw_match_count: u64,

    /// How much wall clock time was consumed when evaluating this rule in the second stage of
    /// matching using `regex`?
    pub stage2_duration: Duration,
}

// -------------------------------------------------------------------------------------------------
// RuleStage2Timer
// -------------------------------------------------------------------------------------------------
pub struct RuleStage2Timer<'a> {
    rule_id: usize,
    start_time: std::time::Instant,
    rule_stats: &'a mut RuleProfile,
}

impl<'a> RuleStage2Timer<'a> {
    #[inline]
    pub fn new(rule_stats: &'a mut RuleProfile, rule_id: usize) -> Self {
        RuleStage2Timer {
            rule_id,
            start_time: Instant::now(),
            rule_stats,
        }
    }
}

impl<'a> Drop for RuleStage2Timer<'a> {
    #[inline]
    fn drop(&mut self) {
        self.rule_stats
            .increment_stage2_duration(self.rule_id, self.start_time.elapsed());
    }
}
