#[derive(Debug, thiserror::Error)]
pub enum GuesserError {
    #[error("libmagic error: {0}")]
    MagicError(#[from] magic::MagicError),
}
