use reqwest::{IntoUrl, Url};

use super::{Auth, Client, Error, Result};

// -------------------------------------------------------------------------------------------------
// ClientBuilder
// -------------------------------------------------------------------------------------------------
pub struct ClientBuilder {
    base_url: reqwest::Url,
    auth: Auth,
}

impl ClientBuilder {
    const USER_AGENT: &str = "noseyparker";

    pub fn new() -> Self {
        ClientBuilder {
            base_url: Url::parse("https://api.github.com").expect("default base URL should parse"),
            auth: Auth::Unauthenticated,
        }
    }

    pub fn base_url<T: IntoUrl>(mut self, url: T) -> Result<Self> {
        self.base_url = url.into_url().map_err(Error::ReqwestError)?;
        Ok(self)
    }

    pub fn auth(mut self, auth: Auth) -> Self {
        self.auth = auth;
        self
    }

    pub fn build(self) -> Result<Client> {
        let inner = reqwest::ClientBuilder::new()
            .user_agent(Self::USER_AGENT)
            .build()
            .map_err(Error::ReqwestError)?;
        Ok(Client {
            base_url: self.base_url,
            auth: self.auth,
            inner,
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
