use url::Url;

mod auth;
mod client;
mod client_builder;
mod error;
mod models;
mod repo_enumerator;
mod result;

pub use auth::Auth;
pub use client::Client;
pub use client_builder::ClientBuilder;
pub use error::Error;
pub use repo_enumerator::{RepoEnumerator, RepoSpecifiers, RepoType};
pub use result::Result;

use progress::Progress;

/// List accessible repository URLs matching the given specifiers.
///
/// This is a high-level wrapper for enumerating GitHub repositories that handles the details of
/// creating an async runtime and a GitHub REST API client.
pub fn enumerate_repo_urls(
    repo_specifiers: &RepoSpecifiers,
    github_url: Url,
    ignore_certs: bool,
    progress: Option<&mut Progress>,
) -> anyhow::Result<Vec<String>> {
    use anyhow::{bail, Context};
    use tracing::{debug, warn};

    let client = ClientBuilder::new()
        .base_url(github_url)
        .context("Failed to set base URL")?
        .personal_access_token_from_env()
        .context("Failed to get GitHub access token from environment")?
        .ignore_certs(ignore_certs)
        .build()
        .context("Failed to initialize GitHub client")?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to initialize async runtime")?;

    let result = runtime.block_on(async {
        // Get rate limit first thing.
        // If there are connectivity issues, this is likely to reveal them quickly.
        //
        // This also makes it a little bit simpler to test this code, as the very first request
        // made by `github repos list` will always be the same, regardless of which repo specifiers
        // are given.
        let rate_limit = client.get_rate_limit().await?;
        debug!("GitHub rate limits: {:?}", rate_limit.rate);

        let repo_enumerator = RepoEnumerator::new(&client);
        let repo_urls = repo_enumerator
            .enumerate_repo_urls(repo_specifiers, progress)
            .await?;
        Ok(repo_urls) // ::<Vec<String>, Error>(repo_urls)
    });

    match result {
        Ok(repo_urls) => Ok(repo_urls),
        Err(err) => {
            if let Error::RateLimited { wait, .. } = err {
                let suggestion = if client.is_authenticated() {
                    ""
                } else {
                    "; consider supplying a GitHub personal access token through the NP_GITHUB_TOKEN environment variable"
                };
                warn!("Rate limit exceeded: must wait for {wait:?} before retrying{}", suggestion);
            }
            bail!(err);
        }
    }
}
