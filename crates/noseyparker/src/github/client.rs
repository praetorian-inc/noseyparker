use chrono::{DateTime, Duration, TimeDelta, TimeZone, Utc};
use reqwest;
use reqwest::{header, header::HeaderValue, StatusCode, Url};
use secrecy::ExposeSecret;

use super::models::{OrganizationShort, Page, RateLimitOverview, Repository, User};
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

    pub fn is_authenticated(&self) -> bool {
        match self.auth {
            Auth::Unauthenticated => false,
            Auth::PersonalAccessToken(_) => true,
        }
    }

    pub async fn get_rate_limit(&self) -> Result<RateLimitOverview> {
        let response = self.get(&["rate_limit"]).await?;
        let body = response.json().await?;
        Ok(body)
    }

    pub async fn get_user(&self, username: &str) -> Result<User> {
        let response = self.get(&["users", username]).await?;
        let body = response.json().await?;
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

    pub async fn get_orgs(&self) -> Result<Page<OrganizationShort>> {
        self.get_paginated_with_params(&["organizations"], &[MAX_PER_PAGE])
            .await
    }

    pub async fn next_page<T>(&self, page: Page<T>) -> Result<Option<Page<T>>>
    where
        T: serde::de::DeserializeOwned,
    {
        self.next_page_inner(page.links.next).await
    }

    async fn next_page_inner<T>(&self, next: Option<Url>) -> Result<Option<Page<T>>>
    where
        T: serde::de::DeserializeOwned,
    {
        match next {
            Some(next) => {
                let response = self.get_url(next).await?;
                Ok(Some(Page::from_response(response).await?))
            }
            None => Ok(None),
        }
    }

    pub async fn get_all<T>(&self, page: Page<T>) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut results = Vec::new();
        let mut next_page = Some(page);
        while let Some(page) = next_page {
            results.extend(page.items.into_iter());
            next_page = self.next_page_inner(page.links.next).await?;
        }
        Ok(results)
    }
}

/// Create a URL from the given base, path parts, and parameters.
///
/// The path parts should not contain slashes.
fn url_from_path_parts_and_params(
    base_url: Url,
    path_parts: &[&str],
    params: &[(&str, &str)],
) -> Result<Url> {
    if base_url.cannot_be_a_base() {
        return Err(Error::UrlBaseError(base_url));
    }

    let mut buf = base_url.path().to_string();
    if !buf.ends_with('/') {
        buf.push('/');
    }

    for (i, p) in path_parts.iter().enumerate() {
        if p.contains('/') {
            return Err(Error::UrlSlashError(p.to_string()));
        }
        if i > 0 {
            // do not add a leading slash for the very first path part, or the result comes out
            // wrong, as it is unintentionally treated as an absolute path
            //
            // https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=c2674663bf5e681b5bdb302d1b050237
            buf.push('/');
        }
        buf.push_str(p);
    }
    let url = base_url.join(&buf)?;
    let url = if params.is_empty() {
        Url::parse(url.as_str())
    } else {
        Url::parse_with_params(url.as_str(), params)
    }?;
    Ok(url)
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    fn make_url(base_url: &str, path_parts: &[&str], params: &[(&str, &str)]) -> Result<Url> {
        let base_url = Url::parse(base_url).unwrap();
        url_from_path_parts_and_params(base_url, path_parts, params)
    }

    fn testcase_ok(inputs: (&str, &[&str], &[(&str, &str)]), expected: &str) {
        let (base_url, path_parts, params) = inputs;
        let actual = make_url(base_url, path_parts, params).unwrap();
        let expected = Url::parse(expected).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn url_from_path_parts_and_params_1() {
        testcase_ok(
            ("https://github.example.com/api/v3", &[], &[]),
            "https://github.example.com/api/v3/",
        );
    }

    #[test]
    fn url_from_path_parts_and_params_2() {
        testcase_ok(
            ("https://github.example.com/api/v3/", &[], &[]),
            "https://github.example.com/api/v3/",
        );
    }

    #[test]
    fn url_from_path_parts_and_params_3() {
        testcase_ok(
            ("https://github.example.com/api/v3", &["SomeUser", "somerepo.git"], &[]),
            "https://github.example.com/api/v3/SomeUser/somerepo.git",
        );
    }

    #[test]
    fn url_from_path_parts_and_params_4() {
        testcase_ok(
            ("https://github.example.com/api/v3/", &["SomeUser", "somerepo.git"], &[]),
            "https://github.example.com/api/v3/SomeUser/somerepo.git",
        );
    }

    #[test]
    fn url_from_path_parts_and_params_5() {
        testcase_ok(
            ("https://api.github.com", &["praetorian-inc", "noseyparker.git"], &[]),
            "https://api.github.com/praetorian-inc/noseyparker.git",
        );
    }

    #[test]
    fn url_from_path_parts_and_params_6() {
        let res =
            make_url("https://api.github.com", &["praetorian-inc", "some/bogus/path/part"], &[]);
        // XXX have to resort to match here because `Error` doesn't have an Eq instance
        match res {
            Err(Error::UrlSlashError(p)) if p == "some/bogus/path/part" => (),
            _ => assert!(false),
        }
    }

    #[test]
    fn url_from_path_parts_and_params_7() {
        let res = make_url("mailto:blah@example.com", &[], &[]);
        // XXX have to resort to match here because `Error` doesn't have an Eq instance
        match res {
            Err(Error::UrlBaseError(p)) if p.as_str() == "mailto:blah@example.com" => (),
            _ => assert!(false),
        }
    }
}

// private implementation
impl Client {
    /// Construct a `Url` from the given path parts and query parameters.
    fn make_url(&self, path_parts: &[&str], params: &[(&str, &str)]) -> Result<Url> {
        url_from_path_parts_and_params(self.base_url.clone(), path_parts, params)
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
        let response = request_builder.send().await?;

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
                let wait =
                    atoi::atoi::<i64>(retry_after.as_bytes()).and_then(TimeDelta::try_seconds);
                let client_error = response.json().await?;
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

                let client_error = response.json().await?;
                return Err(Error::RateLimited { client_error, wait });
            }
        }

        let response = response.error_for_status()?;
        Ok(response)
    }
}
