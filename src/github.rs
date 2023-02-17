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
pub use repo_enumerator::{RepoEnumerator, RepoSpecifiers};
pub use result::Result;

/// List accessible repository URLs matching the given specifiers.
///
/// This is a high-level wrapper for enumerating GitHub repositories that handles the details of
/// creating an async runtime and a GitHub REST API client.
pub fn enumerate_repo_urls(repo_specifiers: &RepoSpecifiers) -> anyhow::Result<Vec<String>> {
    use anyhow::{bail, Context};
    use tracing::{debug, warn};

    let client = ClientBuilder::new()
        .personal_access_token_from_env()
        .context("Failed to load access token from environment")?
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
        let repo_urls = repo_enumerator.enumerate_repo_urls(repo_specifiers).await?;
        Ok(repo_urls) // ::<Vec<String>, Error>(repo_urls)
    });

    match result {
        Ok(repo_urls) => Ok(repo_urls),
        Err(err) => {
            if let Error::RateLimited { wait, .. } = err {
                warn!("Rate limit exceeded: Would need to wait for {wait:?} before retrying");
            }
            bail!(err);
        }
    }
}
