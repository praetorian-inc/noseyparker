// -------------------------------------------------------------------------------------------------
// MatchStats
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Default, Clone)]
pub struct MatcherStats {
    pub blobs_seen: u64,
    pub blobs_scanned: u64,
    pub bytes_seen: u64,
    pub bytes_scanned: u64,

    #[cfg(feature = "rule_profiling")]
    pub rule_stats: crate::rule_profiling::RuleProfile,
}

impl MatcherStats {
    pub fn update(&mut self, other: &Self) {
        self.blobs_seen += other.blobs_seen;
        self.blobs_scanned += other.blobs_scanned;
        self.bytes_seen += other.bytes_seen;
        self.bytes_scanned += other.bytes_scanned;

        #[cfg(feature = "rule_profiling")]
        self.rule_stats.update(&other.rule_stats);
    }
}
