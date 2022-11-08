// -------------------------------------------------------------------------------------------------
// MatchStats
// -------------------------------------------------------------------------------------------------
#[derive(Default, Debug)]
pub struct MatcherStats {
    pub blobs_seen: u64,
    pub blobs_scanned: u64,
    pub bytes_seen: u64,
    pub bytes_scanned: u64,
}

impl MatcherStats {
    pub fn update(&mut self, other: &MatcherStats) {
        self.blobs_seen += other.blobs_seen;
        self.blobs_scanned += other.blobs_scanned;
        self.bytes_seen += other.bytes_seen;
        self.bytes_scanned += other.bytes_scanned;
    }
}
