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

#[allow(dead_code)]
pub struct PrefixContent<T> {
    /// The prefix of the full content
    pub(crate) content: T,

    /// The length of the full content
    pub(crate) full_length: Option<usize>,
}

/// The input to a `Guesser`.
#[allow(dead_code)]
pub struct Input<'a, T> {
    pub(crate) path: Option<&'a Path>,
    pub(crate) content: Content<T>,
}

impl<'a, T> Input<'a, T> {
    /// Create an `Input` from a path without any content. No I/O is performed.
    pub fn from_path_no_io(path: &'a Path) -> Self {
        Self {
            path: Some(path),
            content: Content::None,
        }
    }

    #[inline]
    pub fn path(&self) -> Option<&Path> {
        self.path
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
