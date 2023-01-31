use chrono::{DateTime, Utc, TimeZone, Duration};
use reqwest;
use reqwest::{header, header::HeaderValue, StatusCode, IntoUrl, Url};
use secrecy::{ExposeSecret, SecretString};

pub mod models;

// -------------------------------------------------------------------------------------------------
// Result
// -------------------------------------------------------------------------------------------------
pub type Result<T> = std::result::Result<T, Error>;

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
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RateLimited{..} => write!(f, "request was rate-limited"),
            Error::UrlParseError(e) => write!(f, "error parsing URL: {}", e),
            Error::UrlSlashError(p) => write!(f, "error building URL: component {:?} contains a slash", p),
            Error::ReqwestError(e) => write!(f, "error making request: {}", e),
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
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Auth
// -------------------------------------------------------------------------------------------------
/// Supported forms of authentication
pub enum Auth {
    /// No authentication
    Unauthenticated,

    /// Authenticate with a GitHub Personal Access Token
    PersonalToken(SecretString),
}

// -------------------------------------------------------------------------------------------------
// ClientBuilder
// -------------------------------------------------------------------------------------------------
pub struct ClientBuilder {
    base_url: Option<reqwest::Url>,
    auth: Option<Auth>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        ClientBuilder {
            base_url: None,
            auth: None,
        }
    }

    pub fn base_url<T: IntoUrl>(mut self, url: T) -> Result<Self> {
        self.base_url = Some(url.into_url().map_err(Error::ReqwestError)?);
        Ok(self)
    }

    pub fn auth(mut self, auth: Auth) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn build(self) -> Result<Client> {
        let base_url = self.base_url.unwrap_or_else(|| {
            Url::parse("https://api.github.com").expect("default base URL should parse")
        });
        let auth = self.auth.unwrap_or(Auth::Unauthenticated);
        let inner = reqwest::ClientBuilder::new()
            .user_agent("noseyparker")
            .build()
            .map_err(Error::ReqwestError)?;
        Ok(Client {
            base_url,
            auth,
            inner,
        })
    }
}

// -------------------------------------------------------------------------------------------------
// Client
// -------------------------------------------------------------------------------------------------
pub struct Client {
    base_url: reqwest::Url,
    inner: reqwest::Client,
    auth: Auth,
}

// TODO: deserialization of results
// TODO: debug logging
// TODO: rate limiting support via headers
// TODO: pagination support; per_page query parameter
// TODO: retry combinators?
// TODO: graceful error handling / HTTP response code handling
impl Client {
    pub fn new() -> Result<Self> {
        ClientBuilder::new().build()
    }

    pub async fn rate_limit(&self) -> Result<models::RateLimitOverview> {
        let response = self.get(&["rate_limit"]).await?;
        let body = response.json().await.map_err(Error::ReqwestError)?;
        Ok(body)
    }

    pub async fn user(&self, username: &str) -> Result<models::User> {
        let response = self.get(&["users", username]).await?;
        let body = response.json().await.map_err(Error::ReqwestError)?;
        Ok(body)
    }

//    pub async fn user_repos(&self, user: &models::User) -> Result<models::Page<models::Repository>> {
    pub async fn user_repos(&self, username: &str) -> Result<Vec<models::Repository>> {
        let response = self.get(&["users", username, "repos"]).await?;
        let body = response.json().await.map_err(Error::ReqwestError)?;
        Ok(body)
    }

    async fn get(&self, path_parts: &[&str]) -> Result<reqwest::Response> {
        let url = {
            let mut buf = String::new();
            for p in path_parts {
                buf.push_str("/");
                if p.contains('/') {
                    return Err(Error::UrlSlashError(p.to_string()));
                }
                buf.push_str(p);
            }
            self.base_url.clone().join(&buf).map_err(Error::UrlParseError)?
        };
        println!("GET {}", url);
        let request_builder = self.inner.get(url)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");

        let request_builder = match &self.auth {
            Auth::PersonalToken(token) => {
                request_builder.bearer_auth(token.expose_secret())
            }
            Auth::Unauthenticated => request_builder
        };

        let response = request_builder.send().await.map_err(Error::ReqwestError)?;

        // Check for rate limiting.
        // Instead of using an HTTP 429 response code, GitHub uses 403 and sets the
        // `x-ratelimit-remaining` header to 0.
        if response.status() == StatusCode::FORBIDDEN {
            if let Some(b"0") = response.headers().get("x-ratelimit-remaining").map(HeaderValue::as_bytes) {
                println!("{:#?}", response.headers());
                let date: Option<DateTime<Utc>> = match response.headers().get("date") {
                    Some(v) => match v.to_str() {
                        Ok(v) => DateTime::parse_from_rfc2822(v).ok().map(|v| v.with_timezone(&Utc)),
                        Err(_) => None,
                    }
                    None => None,
                };
                let reset_time: Option<DateTime<Utc>> = match response.headers().get("x-ratelimit-reset") {
                    Some(v) => match v.to_str() {
                        Ok(v) => v.parse::<i64>().ok().and_then(|v| Utc.timestamp_opt(v, 0).single()),
                        Err(_) => None,
                    }
                    None => None,
                };
                // N.B. can convert `wait` to `std::time::Duration` with the `.to_std()` method
                let wait: Option<Duration> = match (date, reset_time) {
                    (Some(t1), Some(t2)) => Some(t2 - t1),
                    _ => None,
                };

                let client_error = response.json().await.map_err(Error::ReqwestError)?;
                return Err(Error::RateLimited {
                    client_error,
                    wait,
                });
            }
        }

        Ok(response.error_for_status().map_err(Error::ReqwestError)?)
    }
}
