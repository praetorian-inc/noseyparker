use mime_guess::MimeGuess;

use crate::{
    error::GuesserError,
    input::Input,
    output::Output,
};

pub struct Guesser {
    #[cfg(feature = "libmagic")]
    magic_cookie: magic::Cookie,
}

// Public Implementation
impl Guesser {
    #[cfg(feature = "libmagic")]
    pub fn new() -> Result<Self, GuesserError> {
        use magic::CookieFlags;
        let flags = CookieFlags::ERROR | CookieFlags::MIME;
        assert!(!flags.contains(CookieFlags::DEBUG));
        let magic_cookie = magic::Cookie::open(flags)?;
        // Load the default database
        magic_cookie.load::<&str>(&[])?;
        Ok(Guesser { magic_cookie })
    }

    #[cfg(not(feature = "libmagic"))]
    pub fn new() -> Result<Self, GuesserError> {
        Ok(Guesser {})
    }

    pub fn guess<'a, T>(&self, input: Input<'a, T>) -> Output
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
