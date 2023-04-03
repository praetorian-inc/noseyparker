mod error;
mod native;
mod wrapper;

pub use error::{AsResult, Error, HyperscanErrorCode};
pub use native::*;
pub use wrapper::{Flag, Pattern, ScanMode, Scratch};
