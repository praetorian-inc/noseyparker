use console::strip_ansi_codes;
use std::borrow::Cow;
use std::fmt::{Display, Formatter, Write};

fn escape_nonprinting(s: &str) -> Cow<'_, str> {
    for (i, ch) in s.char_indices() {
        if !ch.is_whitespace() && ch.is_control() {
            let mut escaped = String::with_capacity(s.len());
            escaped.push_str(&s[..i]);
            for ch in s[i..].chars() {
                if !ch.is_whitespace() && ch.is_control() {
                    write!(escaped, "{}", ch.escape_unicode())
                        .expect("should be able to append escape sequence");
                } else {
                    escaped.push(ch);
                }
            }
            return Cow::Owned(escaped);
        }
    }
    Cow::Borrowed(s)
}

/// A newtype around `&[u8]` that:
///
/// - Decodes from UTF-8, replacing invalid byte sequences with backslash-esacped equivalents
/// - Replaces non-whitespace control characters with `\u{xxxxxx}` escape sequences
/// - Strips out ANSI control sequences
pub struct Escaped<'a>(pub &'a [u8]);

impl Display for Escaped<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let b = String::from_utf8_lossy(self.0);
        let b = escape_nonprinting(&b);
        let b = strip_ansi_codes(&b);
        write!(f, "{b}")
    }
}
