use magic;
use mime_guess::MimeGuess;
use std::io::Read;
use std::path::Path;

pub enum Content<T> {
    /// No content
    None,

    /// An incomplete prefix of the entire contents of a file
    Prefix(PrefixContent<T>),

    /// The entire contents of a file
    Full(T),
}

pub struct PrefixContent<T> {
    /// The prefix of the full content
    content: T,

    /// The length of the full content
    full_length: Option<usize>,
}

/// The input to a `Guesser`.
pub struct Input<'a, T> {
    path: Option<&'a Path>,
    content: Content<T>,
}

impl<'a, T> Input<'a, T> {
    /// Create an `Input` from a path without any content. No I/O is performed.
    pub fn from_path_no_io(path: &'a Path) -> Self {
        Self {
            path: Some(path),
            content: Content::None,
        }
    }
}

impl<'a> Input<'a, &'a [u8]> {
    pub fn from_path_and_bytes(path: &'a Path, bytes: &'a [u8]) -> Self {
        Input {
            path: Some(path),
            content: Content::Full(bytes),
        }
    }

    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Input {
            path: None,
            content: Content::Full(bytes),
        }
    }
}

impl<'a> Input<'a, Vec<u8>> {
    /// Create an `Input` from the given path, reading at most `max_length` bytes of input.
    /// If no `max_length` is given, the entire file contents are read.
    pub fn from_path(path: &'a Path, max_length: Option<usize>) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let expected_len = metadata.len();

        let content = if let Some(max_length) = max_length {
            let f = std::fs::File::open(path)?;
            let mut buf = Vec::with_capacity(max_length);
            let actual_len = f.take(max_length as u64).read_to_end(&mut buf)?;
            if actual_len < expected_len as usize {
                Content::Prefix(PrefixContent {
                    full_length: Some(expected_len as usize),
                    content: buf,
                })
            } else {
                Content::Full(buf)
            }
        } else {
            Content::Full(std::fs::read(path)?)
        };

        Ok(Self {
            path: Some(path),
            content,
        })
    }
}

pub struct Output {
    /// Path-based media type guess
    mime_guess: Option<MimeGuess>,

    magic_guess: Option<String>,
}

impl Output {
    pub fn guessed_types(&self) -> Vec<String> {
        let mut guessed_types = Vec::new();

        if let Some(mime_guess) = self.mime_guess {
            guessed_types.extend(mime_guess.iter().map(|m| m.to_string()));
        }

        if let Some(magic_guess) = &self.magic_guess {
            guessed_types.push(magic_guess.to_string())
        }

        guessed_types
    }
}

pub struct Guesser {
    magic_cookie: magic::Cookie,
}

// Public Implementation
impl Guesser {
    pub fn new() -> Self {
        let magic_cookie = magic::Cookie::open(magic::CookieFlags::ERROR).expect("FIXME");
        // Load the default database
        magic_cookie.load::<&str>(&[]).expect("FIXME");
        Guesser { magic_cookie }
    }

    pub fn guess<'a, T>(&self, input: Input<'a, T>) -> Output
    where
        T: AsRef<[u8]>,
    {
        let mime_guess = input.path.map(MimeGuess::from_path);

        let magic_guess = match &input.content {
            Content::None => None,
            Content::Prefix(PrefixContent { content, .. }) | Content::Full(content) => {
                self.magic_cookie.buffer(content.as_ref()).ok()
            }
        };

        Output {
            mime_guess,
            magic_guess,
        }
    }
}

impl Default for Guesser {
    fn default() -> Self {
        Self::new()
    }
}
