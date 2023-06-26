use magic;
pub use mime::Mime;
use mime_guess::MimeGuess;

mod input;
pub use input::{Content, PrefixContent, Input};

mod output;
pub use output::Output;

mod error;
pub use error::GuesserError;


pub struct Guesser {
    magic_cookie: magic::Cookie,
}

// Public Implementation
impl Guesser {
    pub fn new() -> Result<Self, GuesserError> {
        use magic::CookieFlags;
        let magic_cookie = magic::Cookie::open(CookieFlags::ERROR | CookieFlags::MIME)?;
        // Load the default database
        magic_cookie.load::<&str>(&[])?;
        Ok(Guesser { magic_cookie })
    }

    pub fn guess<'a, T>(&self, input: Input<'a, T>) -> Output
    where
        T: AsRef<[u8]>,
    {
        let mime_guess = input.path.map(MimeGuess::from_path);

        let magic_guess = match &input.content {
            Content::None => None,
            Content::Prefix(PrefixContent { content, .. }) | Content::Full(content) => {
                match self.magic_cookie.buffer(content.as_ref()) {
                    Ok(m) => m.parse().ok(),
                    _ => None,
                }
            }
        };

        Output {
            mime_guess,
            magic_guess,
        }
    }
}
