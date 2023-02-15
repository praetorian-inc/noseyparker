use super::models::Repository;
use super::{Client, Result};

/// A `RepoEnumerator` provides higher-level functionality on top of the GitHub REST API to list
/// repositories belonging to specific users or organizations.
pub struct RepoEnumerator<'c> {
    client: &'c Client,
}

impl<'c> RepoEnumerator<'c> {
    pub fn new(client: &'c Client) -> Self {
        Self { client }
    }

    /// Enumerate the accessible repositories that belong to the given user.
    pub async fn enumerate_user_repos(&self, username: &str) -> Result<Vec<Repository>> {
        let repo_page = self.client.get_user_repos(username).await?;
        self.client.get_all(repo_page).await
    }

    /// Enumerate the accessible repositories that belong to the given organization.
    pub async fn enumerate_org_repos(&self, orgname: &str) -> Result<Vec<Repository>> {
        let repo_page = self.client.get_org_repos(orgname).await?;
        self.client.get_all(repo_page).await
    }

    /// Enumerate the repository clone URLs found from the according to the given `RepoSpecifiers`,
    /// collecting the union of specified repository URLs.
    ///
    /// The resulting URLs are sorted and deduplicated.
    pub async fn enumerate_repo_urls(
        &self,
        repo_specifiers: &RepoSpecifiers,
    ) -> Result<Vec<String>> {
        let mut repo_urls = Vec::new();

        for username in &repo_specifiers.user {
            repo_urls.extend(
                self.enumerate_user_repos(username)
                    .await?
                    .into_iter()
                    .map(|r| r.clone_url),
            );
        }

        for orgname in &repo_specifiers.organization {
            repo_urls.extend(
                self.enumerate_org_repos(orgname)
                    .await?
                    .into_iter()
                    .map(|r| r.clone_url),
            );
        }

        repo_urls.sort();
        repo_urls.dedup();

        Ok(repo_urls)
    }
}

/// Specifies a set of GitHub usernames and/or organization names.
#[derive(Debug)]
pub struct RepoSpecifiers {
    pub user: Vec<String>,
    pub organization: Vec<String>,
}

impl RepoSpecifiers {
    pub fn is_empty(&self) -> bool {
        self.user.is_empty() && self.organization.is_empty()
    }
}
