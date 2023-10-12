#[derive(Debug, thiserror::Error)]
pub enum GuesserError {
    #[cfg(feature = "libmagic")]
    #[error("libmagic error: {0}")]
    MagicError(String),
}
