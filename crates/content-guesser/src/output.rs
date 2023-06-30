use mime::Mime;
use mime_guess::MimeGuess;

#[derive(Debug)]
pub struct Output {
    /// Path-based media type guess
    pub(crate) mime_guess: Option<MimeGuess>,

    /// Content-based media type guess
    pub(crate) magic_guess: Option<Mime>,
}

impl Output {
    /// Get the path-based media type guess
    #[inline]
    pub fn path_guess(&self) -> Option<Mime> {
        self.mime_guess.and_then(|g| g.first())
    }

    /// Get the content-based media type guess
    #[inline]
    pub fn content_guess(&self) -> Option<Mime> {
        self.magic_guess.clone()
    }

    /// Get the guessed mime type that is considered to be the best.
    ///
    /// If a content-based guess is available, that is used.
    /// Otherwise, the path-based guess is used.
    pub fn best_guess(&self) -> Option<Mime> {
        self.content_guess().or_else(|| self.path_guess())
    }
}
