use reqwest::{IntoUrl, Url};
use tracing::debug;

use super::{Auth, Client, Error, Result};

// -------------------------------------------------------------------------------------------------
// ClientBuilder
// -------------------------------------------------------------------------------------------------
pub struct ClientBuilder {
    base_url: reqwest::Url,
    auth: Auth,
    ignore_certs: bool,
}

impl ClientBuilder {
    /// The user agent string sent when accessing the GitHub REST API
    const USER_AGENT: &'static str = "noseyparker";

    /// Create a new `ClientBuilder` that uses unauthenticated access to <https://api.github.com>.
    pub fn new() -> Self {
        ClientBuilder {
            base_url: Url::parse("https://api.github.com").expect("default base URL should parse"),
            auth: Auth::Unauthenticated,
            ignore_certs: false,
        }
    }

    /// Use the specified base URL.
    pub fn base_url<T: IntoUrl>(mut self, url: T) -> Result<Self> {
        self.base_url = url.into_url()?;
        Ok(self)
    }

    /// Use the given authentication mechanism.
    pub fn auth(mut self, auth: Auth) -> Self {
        self.auth = auth;
        self
    }

    /// Ignore validation of TLS certs.
    pub fn ignore_certs(mut self, ignore_certs: bool) -> Self {
        self.ignore_certs = ignore_certs;
        self
    }

    /// Load an optional personal access token token from the `NP_GITHUB_TOKEN` environment variable.
    /// If that variable is not set, unauthenticated access is used.
    pub fn personal_access_token_from_env(self) -> Result<Self> {
        self.personal_access_token_from_env_var("NP_GITHUB_TOKEN")
    }

    fn personal_access_token_from_env_var(mut self, env_var_name: &str) -> Result<Self> {
        match std::env::var(env_var_name) {
            Err(std::env::VarError::NotPresent) => {
                debug!("No GitHub access token provided; using unauthenticated API access.");
            }
            Err(std::env::VarError::NotUnicode(_s)) => {
                return Err(Error::InvalidTokenEnvVar(env_var_name.to_string()));
            }
            Ok(val) => {
                debug!(
                    "Using GitHub personal access token from {env_var_name} environment variable"
                );
                self.auth = Auth::PersonalAccessToken(secrecy::SecretString::from(val));
            }
        }
        Ok(self)
    }

    /// Build a `Client` from this `ClientBuilder`.
    pub fn build(self) -> Result<Client> {
        let inner = reqwest::ClientBuilder::new()
            .user_agent(Self::USER_AGENT)
            .danger_accept_invalid_certs(self.ignore_certs)
            .build()?;
        Ok(Client {
            base_url: self.base_url,
            auth: self.auth,
            inner,
        })
    }
}

impl Default for ClientBuilder {
    /// Equivalent to `ClientBuilder::new()`.
    fn default() -> Self {
        Self::new()
    }
}
