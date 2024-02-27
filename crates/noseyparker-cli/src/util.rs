#![allow(dead_code)]

/// A utility type to generate properly pluralized count expressions in log messages,
/// e.g., "1 rule" or "7 rules", without copying data.
pub enum Counted<'a> {
    Regular {
        singular: &'a str,
        count: usize,
    },
    Explicit {
        singular: &'a str,
        count: usize,
        plural: &'a str,
    },
}

impl<'a> Counted<'a> {
    /// Create a new `Counted` value with the given count, singular, and plural values.
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
