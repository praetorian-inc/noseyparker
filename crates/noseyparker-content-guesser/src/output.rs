use mime::Mime;
use mime_guess::MimeGuess;

#[derive(Debug)]
pub struct Output {
    /// Path-based media type guess
    pub(crate) mime_guess: Option<MimeGuess>,

    pub(crate) magic_guess: Option<Mime>,
}

impl Output {
    pub fn guessed_types(&self) -> Vec<Mime> {
        let mut guessed_types = Vec::new();

        if let Some(mime_guess) = self.mime_guess {
            guessed_types.extend(mime_guess);
        }

        if let Some(magic_guess) = &self.magic_guess {
            guessed_types.push(magic_guess.clone());
        }

        guessed_types
    }
}
