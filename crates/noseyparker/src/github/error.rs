use super::models;
use chrono::Duration;

// -------------------------------------------------------------------------------------------------
// Error
// -------------------------------------------------------------------------------------------------
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("request was rate-limited: {}", .client_error.message)]
    RateLimited {
        /// The client error returned by GitHub
        client_error: models::ClientError,

        /// The duration to wait until trying again
        wait: Option<Duration>,
    },

    #[error("invalid base url: {0}")]
    UrlBaseError(url::Url),

    #[error("error parsing URL: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("error building URL: component {0:?} contains a slash")]
    UrlSlashError(String),

    #[error("error making request: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("error loading token: ill-formed value of {0} environment variable")]
    InvalidTokenEnvVar(String),
}
