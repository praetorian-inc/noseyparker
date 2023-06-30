pub use mime::Mime;

mod input;
pub use input::{Content, PrefixContent, Input};

mod output;
pub use output::Output;

mod error;
pub use error::GuesserError;

mod guesser;
pub use guesser::Guesser;
