use chrono::{DateTime, Duration, TimeZone, Utc};
use reqwest;
use reqwest::{header, header::HeaderValue, StatusCode, Url};
use secrecy::ExposeSecret;

use super::models::{Page, RateLimitOverview, Repository, User};
use super::{Auth, ClientBuilder, Error, Result};

// TODO: debug logging
// TODO: retry combinators, to handle rate limiting and HTTP errors

// -------------------------------------------------------------------------------------------------
// Client
// -------------------------------------------------------------------------------------------------
pub struct Client {
    pub(super) base_url: Url,
    pub(super) inner: reqwest::Client,
    pub(super) auth: Auth,
}

const MAX_PER_PAGE: (&str, &str) = ("per_page", "100");

impl Client {
    pub fn new() -> Result<Self> {
        ClientBuilder::new().build()
    }

    pub async fn get_rate_limit(&self) -> Result<RateLimitOverview> {
        let response = self.get(&["rate_limit"]).await?;
        let body = response.json().await.map_err(Error::ReqwestError)?;
        Ok(body)
    }

    pub async fn get_user(&self, username: &str) -> Result<User> {
        let response = self.get(&["users", username]).await?;
        let body = response.json().await.map_err(Error::ReqwestError)?;
        Ok(body)
    }

    pub async fn get_user_repos(&self, username: &str) -> Result<Page<Repository>> {
        let response = self
            .get_with_params(&["users", username, "repos"], &[MAX_PER_PAGE])
            .await?;
        let body = Page::from_response(response).await?;
        Ok(body)
    }

    pub async fn get_org_members(&self, orgname: &str) -> Result<Page<User>> {
        self.get_paginated_with_params(&["orgs", orgname, "members"], &[MAX_PER_PAGE])
            .await
    }

    pub async fn get_org_repos(&self, orgname: &str) -> Result<Page<Repository>> {
        self.get_paginated_with_params(&["orgs", orgname, "repos"], &[MAX_PER_PAGE])
            .await
    }

    pub async fn next_page<T>(&self, page: Page<T>) -> Result<Option<Page<T>>>
    where
        T: serde::de::DeserializeOwned,
    {
        if let Some(next) = page.links.next {
            let response = self.get_url(next).await?;
            Ok(Some(Page::from_response(response).await?))
        } else {
            Ok(None)
        }
    }
}

// private implementation
impl Client {
    /// Construct a `Url` from the given path parts and query parameters.
    fn make_url(&self, path_parts: &[&str], params: &[(&str, &str)]) -> Result<Url> {
        // XXX Surely this can be done better
        let mut buf = String::new();
        for p in path_parts {
            buf.push('/');
            if p.contains('/') {
                return Err(Error::UrlSlashError(p.to_string()));
            }
            buf.push_str(p);
        }
        let url = self
            .base_url
            .clone()
            .join(&buf)
            .map_err(Error::UrlParseError)?;
        let url = if params.is_empty() {
            Url::parse(url.as_str()).map_err(Error::UrlParseError)?
        } else {
            Url::parse_with_params(url.as_str(), params).map_err(Error::UrlParseError)?
        };
        Ok(url)
    }

    async fn get(&self, path_parts: &[&str]) -> Result<reqwest::Response> {
        self.get_with_params(path_parts, &[]).await
    }

    async fn get_with_params(
        &self,
        path_parts: &[&str],
        params: &[(&str, &str)],
    ) -> Result<reqwest::Response> {
        let url = self.make_url(path_parts, params)?;
        self.get_url(url).await
    }

    async fn get_paginated_with_params<T>(
        &self,
        path_parts: &[&str],
        params: &[(&str, &str)],
    ) -> Result<Page<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self.get_with_params(path_parts, params).await?;
        Page::from_response(response).await
    }

    async fn get_url(&self, url: Url) -> Result<reqwest::Response> {
        // build request, handling authentication if any
        let request_builder = self
            .inner
            .get(url)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        let request_builder = match &self.auth {
            Auth::PersonalAccessToken(token) => request_builder.bearer_auth(token.expose_secret()),
            Auth::Unauthenticated => request_builder,
        };

        // send request and wait for response
        let response = request_builder.send().await.map_err(Error::ReqwestError)?;

        // Check for rate limiting.
        //
        // Instead of using an HTTP 429 response code, GitHub uses 403 and sets the
        // `x-ratelimit-remaining` header to 0.
        //
        // Also from the GitHub docs on secondary rate limits:
        //
        //     If the Retry-After response header is present, retry your request after the time
        //     specified in the header. The value of the Retry-After header will always be an
        //     integer, representing the number of seconds you should wait before making
        //     requests again. For example, Retry-After: 30 means you should wait 30 seconds
        //     before sending more requests.
        //
        //     Otherwise, retry your request after the time specified by the x-ratelimit-reset
        //     header. The x-ratelimit-reset header will always be an integer representing the
        //     time at which the current rate limit window resets in UTC epoch seconds.
        if response.status() == StatusCode::FORBIDDEN {
            if let Some(retry_after) = response.headers().get("Retry-After") {
                let wait = atoi::atoi::<i64>(retry_after.as_bytes()).map(Duration::seconds);
                let client_error = response.json().await.map_err(Error::ReqwestError)?;
                return Err(Error::RateLimited { client_error, wait });
            }

            if let Some(b"0") = response
                .headers()
                .get("x-ratelimit-remaining")
                .map(HeaderValue::as_bytes)
            {
                let wait = || -> Option<Duration> {
                    let date = response.headers().get("date")?.to_str().ok()?;
                    let date = DateTime::parse_from_rfc2822(date).ok()?.with_timezone(&Utc);

                    let reset_time = response
                        .headers()
                        .get("x-ratelimit-reset")?
                        .to_str()
                        .ok()?
                        .parse::<i64>()
                        .ok()?;
                    let reset_time = Utc.timestamp_opt(reset_time, 0).single()?;

                    Some(reset_time - date)
                }();

                let client_error = response.json().await.map_err(Error::ReqwestError)?;
                return Err(Error::RateLimited { client_error, wait });
            }
        }

        response.error_for_status().map_err(Error::ReqwestError)
    }
}
