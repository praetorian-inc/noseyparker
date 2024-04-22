use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter};
use std::path::Path;

/// A utility type to generate properly pluralized count expressions in log messages,
/// e.g., "1 rule" or "7 rules", without copying data.
pub enum Counted<'a> {
    Regular {
        singular: &'a str,
        count: usize,
    },
    #[allow(dead_code)]
    Explicit {
        singular: &'a str,
        count: usize,
        plural: &'a str,
    },
}

impl<'a> Counted<'a> {
    /// Create a new `Counted` value with the given count, singular, and plural values.
    #[allow(dead_code)]
    pub fn new(count: usize, singular: &'a str, plural: &'a str) -> Self {
        Counted::Explicit {
            singular,
            plural,
            count,
        }
    }

    /// Create a new `Counted` value with the given count and singular form, which is pluralized by
    /// adding an `s`.
    pub fn regular(count: usize, singular: &'a str) -> Self {
        Counted::Regular { singular, count }
    }
}

impl<'a> std::fmt::Display for Counted<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Counted::Explicit {
                singular,
                plural,
                count,
            } => {
                if count == 1 {
                    write!(f, "1 {}", singular)
                } else {
                    write!(f, "{} {}", count, plural)
                }
            }

            Counted::Regular { singular, count } => {
                if count == 1 {
                    write!(f, "1 {}", singular)
                } else {
                    write!(f, "{} {}s", count, singular)
                }
            }
        }
    }
}

/// Get a buffered writer for the file at the specified output destination, or stdout if not specified.
pub fn get_writer_for_file_or_stdout<P: AsRef<Path>>(
    path: Option<P>,
) -> std::io::Result<Box<dyn std::io::Write>> {
    match path.as_ref() {
        None => Ok(Box::new(BufWriter::new(stdout()))),
        Some(p) => {
            let f = File::create(p)?;
            Ok(Box::new(BufWriter::new(f)))
        }
    }
}

/// Get a buffered reader for the file at the specified input source, or stdin if not specified.
pub fn get_reader_for_file_or_stdin<P: AsRef<Path>>(
    path: Option<P>,
) -> std::io::Result<Box<dyn std::io::Read>> {
    match path.as_ref() {
        None => Ok(Box::new(BufReader::new(stdin()))),
        Some(p) => {
            let f = File::open(p)?;
            Ok(Box::new(BufReader::new(f)))
        }
    }
}
