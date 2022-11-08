use std::path::PathBuf;

// -------------------------------------------------------------------------------------------------
// Provenance
// -------------------------------------------------------------------------------------------------
/// `Provenance` indicates where a particular blob or match was found when scanning.
#[derive(Debug, Clone)]
pub enum Provenance {
    FromFile(PathBuf),
    FromGitRepo(PathBuf),
}
