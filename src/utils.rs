const SIZEOF_PREFIXES: [(usize, &str); 5] = [
    (1024 * 1024 * 1024 * 1024 * 1024, "PiB"),
    (1024 * 1024 * 1024 * 1024, "TiB"),
    (1024 * 1024 * 1024, "GiB"),
    (1024 * 1024, "MiB"),
    (1024, "KiB"),
];

pub fn sizeof_fmt(bytes: usize) -> String {
    let (d, unit) = SIZEOF_PREFIXES
        .iter()
        .find(|(v, _t)| bytes >= *v)
        .unwrap_or(&(1, "B"));
    let v = bytes as f64 / *d as f64;
    format!("{:.2} {}", v, unit)
}

const DURATION_PREFIXES: [(f64, &str); 4] = [
    ((60 * 60 * 24 * 7) as f64, "weeks"),
    ((60 * 60 * 24) as f64, "days"),
    ((60 * 60) as f64, "hours"),
    (60_f64, "minutes"),
];

pub fn duration_fmt(secs: f64) -> String {
    let (d, unit) = DURATION_PREFIXES
        .iter()
        .find(|(v, _t)| secs >= *v)
        .unwrap_or(&(1.0, "seconds"));
    let v = secs / *d as f64;
    format!("{:.2} {}", v, unit)
}

/// Strip ANSI escape sequences from the input and convert it on a best-effort to UTF-8.
// FIXME: also strip out non-printing characters, like NUL
// FIXME: instead of calling `strip_ansi`, use strip_ansi_escapes::Writer to avoid allocating; see https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=6067bfa6c5a855a52752bd28f0df2e8e for ideas
#[inline]
pub fn decode_utf8_lossy_escape(input: &[u8]) -> String {
    use strip_ansi_escapes::strip;
    let input = strip(input).expect("stripping ANSI sequences should not fail");
    String::from_utf8_lossy(&input).into_owned()
}
