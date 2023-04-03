use core::ops::Range;
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------------------------------------
// OffsetPoint
// -------------------------------------------------------------------------------------------------
/// A single-point location within a bytestring
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Copy, Clone)]
pub struct OffsetPoint(pub usize);

impl OffsetPoint {
    /// Create a new `OffsetPoint` at the given byte offset.
    #[inline]
    pub fn new(idx: usize) -> Self {
        OffsetPoint(idx)
    }
}

// -------------------------------------------------------------------------------------------------
// OffsetSpan
// -------------------------------------------------------------------------------------------------
/// A non-empty span within a bytestring
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct OffsetSpan {
    pub start: usize,
    pub end: usize,
}

impl OffsetSpan {
    /// Create a new `OffsetSpan` at the given start and end.
    /// This is a half-open interval: `[start, end)`.
    #[inline]
    pub fn from_offsets(start: OffsetPoint, end: OffsetPoint) -> Self {
        OffsetSpan {
            start: start.0,
            end: end.0,
        }
    }

    /// Create a new `OffsetSpan` from the given `Range<usize>`.
    #[inline]
    pub fn from_range(range: Range<usize>) -> Self {
        OffsetSpan {
            start: range.start,
            end: range.end,
        }
    }

    /// Return the length in bytes of this `OffsetSpan`.
    #[inline]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Is the given span empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Does this `OffsetSpan` entirely contain the other?
    #[inline]
    pub fn fully_contains(&self, other: &OffsetSpan) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

// -------------------------------------------------------------------------------------------------
// SourcePoint
// -------------------------------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy, Clone)]
pub struct SourcePoint {
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for SourcePoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

// -------------------------------------------------------------------------------------------------
// SourceSpan
// -------------------------------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
// FIXME: is this a half-open or closed interval?  Clarify this.
pub struct SourceSpan {
    pub start: SourcePoint,
    pub end: SourcePoint,
}

impl std::fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

// -------------------------------------------------------------------------------------------------
// LocationMapping
// -------------------------------------------------------------------------------------------------
/// A translation table from byte offsets to source offsets
pub struct LocationMapping {
    offset_to_source: Vec<SourcePoint>,
}

// FIXME: add round-tripping property tests
// FIXME: add benchmarks; this code seems very slow
impl LocationMapping {
    /// Create a new location mapping from the given input.
    pub fn new(input: &[u8]) -> Self {
        let mut column = 0;
        let mut line = 1;
        let offset_to_source = input
            .iter()
            .map(|b| {
                match b {
                    b'\r' => {
                        column = 0;
                    }
                    b'\n' => {
                        line += 1;
                        column = 0;
                    }
                    _ => {
                        column += 1;
                    }
                }
                SourcePoint { line, column }
            })
            .collect();
        LocationMapping { offset_to_source }
    }

    /// Get the `SourcePoint` corresponding to the given `OffsetPoint`.
    /// Panics if the given `OffsetPoint` is not valid for this `LocationMapping`.
    pub fn get_source_point(&self, point: &OffsetPoint) -> SourcePoint {
        self.offset_to_source[point.0]
    }

    /// Get the `SourceSpan` corresponding to the given `OffsetSpan`.
    /// Panics if the given `OffsetSpan` is not valid for this `LocationMapping`.
    pub fn get_source_span(&self, span: &OffsetSpan) -> SourceSpan {
        let start = self.offset_to_source[span.start];
        let end = self.offset_to_source[span.end.saturating_sub(1)];  // FIXME: is this right?
        // let end = self.offset_to_source[span.end];
        SourceSpan{ start, end }
    }
}


// -------------------------------------------------------------------------------------------------
// Location
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Location {
    pub offset_span: OffsetSpan,
    pub source_span: SourceSpan,
}
