use super::models::{OrganizationShort, Repository};
use super::{Client, Result};

use progress::Progress;

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

    /// Enumerate the accessible repositories that belong to the given organization.
    pub async fn enumerate_instance_orgs(&self) -> Result<Vec<OrganizationShort>> {
        let org_page = self.client.get_orgs().await?;
        self.client.get_all(org_page).await
    }

    /// Enumerate the repository clone URLs found from the according to the given `RepoSpecifiers`,
    /// collecting the union of specified repository URLs.
    ///
    /// The resulting URLs are sorted and deduplicated.
    pub async fn enumerate_repo_urls(
        &self,
        repo_specifiers: &RepoSpecifiers,
        mut progress: Option<&mut Progress>,
    ) -> Result<Vec<String>> {
        let mut repo_urls = Vec::new();

        for username in &repo_specifiers.user {
            let mut to_add = self.enumerate_user_repos(username).await?;
            to_add.retain(|r| repo_specifiers.repo_filter.filter(r));
            if let Some(progress) = progress.as_mut() {
                progress.inc(to_add.len() as u64);
            }
            repo_urls.extend(to_add.into_iter().map(|r| r.clone_url));
        }

        let instance_orgs: Vec<_> = if repo_specifiers.all_organizations {
            self.enumerate_instance_orgs()
                .await?
                .into_iter()
                .map(|o| o.login)
                .collect()
        } else {
            Default::default()
        };
        let orgs: Vec<&String> = repo_specifiers
            .organization
            .iter()
            .chain(instance_orgs.iter())
            .collect();

        for orgname in orgs {
            let mut to_add = self.enumerate_org_repos(orgname).await?;
            to_add.retain(|r| repo_specifiers.repo_filter.filter(r));
            if let Some(progress) = progress.as_mut() {
                progress.inc(to_add.len() as u64);
            }
            repo_urls.extend(to_add.into_iter().map(|r| r.clone_url));
        }

        repo_urls.sort();
        repo_urls.dedup();

        Ok(repo_urls)
    }
}

/// Specifies which GitHub repositories to select.
#[derive(Debug)]
pub enum RepoType {
    /// Select both source repositories and fork repositories
    All,

    /// Only source repositories, i.e., ones that are forks
    Source,

    /// Only fork repositories
    Fork,
}

impl RepoType {
    fn filter(&self, repo: &Repository) -> bool {
        match self {
            RepoType::All => true,
            RepoType::Source => !repo.fork,
            RepoType::Fork => repo.fork,
        }
    }
}

/// Specifies a set of GitHub usernames and/or organization names.
#[derive(Debug)]
pub struct RepoSpecifiers {
    pub user: Vec<String>,
    pub organization: Vec<String>,
    pub all_organizations: bool,
    pub repo_filter: RepoType,
}

impl RepoSpecifiers {
    pub fn is_empty(&self) -> bool {
        self.user.is_empty() && self.organization.is_empty() && !self.all_organizations
    }
}
