use chrono::Duration;
use super::models;

// -------------------------------------------------------------------------------------------------
// Error
// -------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub enum Error {
    RateLimited {
        /// The client error returned by GitHub
        client_error: models::ClientError,

        /// The duration to wait until trying again
        wait: Option<Duration>,
    },
    UrlParseError(url::ParseError),
    UrlSlashError(String),
    ReqwestError(reqwest::Error),
    InvalidTokenEnvVar(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RateLimited{client_error, ..} => write!(f, "request was rate-limited: {}", client_error.message),
            Error::UrlParseError(e) => write!(f, "error parsing URL: {e}"),
            Error::UrlSlashError(p) => write!(f, "error building URL: component {p:?} contains a slash"),
            Error::ReqwestError(e) => write!(f, "error making request: {e}"),
            Error::InvalidTokenEnvVar(v) => write!(f, "error loading token: ill-formed value of {v} environment variable"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::RateLimited{..} => None,
            Error::UrlParseError(e) => Some(e),
            Error::UrlSlashError(_) => None,
            Error::ReqwestError(e) => Some(e),
            Error::InvalidTokenEnvVar(_) => None,
        }
    }
}
