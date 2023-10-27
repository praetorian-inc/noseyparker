use anyhow::Result;

/// A trait for things that can be output as a document.
///
/// This trait is used to factor output-related code, such as friendly handling of buffering, into
/// one place.
pub trait Reportable {
    type Format;

    fn report<W: std::io::Write>(&self, format: Self::Format, writer: W) -> Result<()>;
}
