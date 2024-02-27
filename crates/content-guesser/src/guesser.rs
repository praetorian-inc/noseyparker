use mime_guess::MimeGuess;

use crate::{error::GuesserError, input::Input, output::Output};

pub struct Guesser {
    #[cfg(feature = "libmagic")]
    magic_cookie: magic::cookie::Cookie<magic::cookie::Load>,
}

// Public Implementation
impl Guesser {
    #[cfg(feature = "libmagic")]
    pub fn new() -> Result<Self, GuesserError> {
        use magic::cookie::Flags;
        let flags = Flags::ERROR | Flags::MIME;
        assert!(!flags.contains(Flags::DEBUG));
        let magic_cookie =
            magic::Cookie::open(flags).map_err(|e| GuesserError::MagicError(e.to_string()))?;
        // Load the default database
        let magic_cookie = magic_cookie
            .load(&Default::default())
            .map_err(|e| GuesserError::MagicError(e.to_string()))?;
        Ok(Guesser { magic_cookie })
    }

    #[cfg(not(feature = "libmagic"))]
    pub fn new() -> Result<Self, GuesserError> {
        Ok(Guesser {})
    }

    pub fn guess<T>(&self, input: Input<T>) -> Output
    where
        T: AsRef<[u8]>,
    {
        let mime_guess = input.path.map(MimeGuess::from_path);

        #[cfg(feature = "libmagic")]
        let magic_guess = {
            use crate::input::{Content, PrefixContent};
            match &input.content {
                Content::None => None,
                Content::Prefix(PrefixContent { content, .. }) | Content::Full(content) => {
                    match self.magic_cookie.buffer(content.as_ref()) {
                        Ok(m) => m.parse().ok(),
                        _ => None,
                    }
                }
            }
        };
        #[cfg(not(feature = "libmagic"))]
        let magic_guess = None;

        Output {
            mime_guess,
            magic_guess,
        }
    }
}
