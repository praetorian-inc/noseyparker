use serde::Serialize;

// -------------------------------------------------------------------------------------------------
// FindingSummary
// -------------------------------------------------------------------------------------------------

/// A summary of matches in a `Datastore`.
#[derive(Serialize)]
pub struct FindingSummary(pub Vec<FindingSummaryEntry>);

#[derive(Serialize)]
pub struct FindingSummaryEntry {
    /// The rule name of this entry
    pub rule_name: String,

    /// The number of findings with this rule
    pub distinct_count: usize,

    /// The number of matches with this rule
    pub total_count: usize,

    /// The number of findings with this rule with the `accept` status
    pub accept_count: usize,

    /// The number of findings with this rule with the `reject` status
    pub reject_count: usize,

    /// The number of findings with this rule with a mixed status, i.e., both `reject` and `accept`
    /// status
    pub mixed_count: usize,

    /// The number of findings with this rule that have no assigned status
    pub unlabeled_count: usize,
}
