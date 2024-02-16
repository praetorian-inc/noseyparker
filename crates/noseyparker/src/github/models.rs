use serde::Deserialize;

pub mod page;
pub use page::Page;

// -------------------------------------------------------------------------------------------------
// ClientError
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct ClientError {
    pub message: String,
    pub documentation_url: Option<String>,
    pub errors: Option<Vec<Error>>,
}

// -------------------------------------------------------------------------------------------------
// Error
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct Error {
    pub resource: String,
    pub field: String,
    pub code: ErrorCode,
}

// -------------------------------------------------------------------------------------------------
// ErrorCode
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub enum ErrorCode {
    Missing,
    MissingField,
    Invalid,
    AlreadyExists,
    Unprocessable,
}

// -------------------------------------------------------------------------------------------------
// RateLimit
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct RateLimitOverview {
    pub resources: Resources,
    pub rate: Rate,
}

// -------------------------------------------------------------------------------------------------
// Resource
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct Resources {
    pub core: Rate,
    pub search: Rate,
    pub graphql: Option<Rate>,
    pub source_import: Option<Rate>,
    pub integration_manifest: Option<Rate>,
    pub code_scanning_upload: Option<Rate>,
    pub actions_runner_registration: Option<Rate>,
    pub scim: Option<Rate>,
    pub dependency_snapshots: Option<Rate>,
}

// -------------------------------------------------------------------------------------------------
// Rate
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct Rate {
    pub limit: i64,
    pub remaining: i64,
    pub reset: i64,
    pub used: i64,
}

// -------------------------------------------------------------------------------------------------
// User
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
    pub id: i64,
    pub node_id: String,
    pub avatar_url: String,
    pub gravatar_id: Option<String>,
    pub url: String,
    pub html_url: String,
    pub followers_url: String,
    pub following_url: String,
    pub gists_url: String,
    pub starred_url: String,
    pub subscriptions_url: String,
    pub organizations_url: String,
    pub repos_url: String,
    pub events_url: String,
    pub received_events_url: String,
    #[serde(rename = "type")]
    pub user_type: String,
    pub site_admin: bool,
    pub name: Option<String>,
    pub company: Option<String>,
    pub blog: Option<String>,
    pub location: Option<String>,
    pub email: Option<String>,
    pub hireable: Option<bool>,
    pub bio: Option<String>,
    pub twitter_username: Option<String>,
    pub public_repos: i64,
    pub public_gists: i64,
    pub followers: i64,
    pub following: i64,
    pub created_at: String,
    pub updated_at: String,
    // pub plan: Option<Box<crate::models::PublicUserPlan>>,
    pub suspended_at: Option<String>,
    pub private_gists: Option<i64>,
    pub total_private_repos: Option<i64>,
    pub owned_private_repos: Option<i64>,
    pub disk_usage: Option<i64>,
    pub collaborators: Option<i64>,

    pub business_plus: Option<bool>,
    pub ldap_dn: Option<String>,
    pub two_factor_authentication: Option<bool>,
}

// -------------------------------------------------------------------------------------------------
// Repository
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub node_id: String,
    pub name: String,
    pub full_name: String,
    // pub owner: Box<crate::models::SimpleUser1>,
    pub private: bool,
    pub html_url: String,
    pub description: Option<String>,
    pub fork: bool,
    pub url: String,
    pub archive_url: String,
    pub assignees_url: String,
    pub blobs_url: String,
    pub branches_url: String,
    pub collaborators_url: String,
    pub comments_url: String,
    pub commits_url: String,
    pub compare_url: String,
    pub contents_url: String,
    pub contributors_url: String,
    pub deployments_url: String,
    pub downloads_url: String,
    pub events_url: String,
    pub forks_url: String,
    pub git_commits_url: String,
    pub git_refs_url: String,
    pub git_tags_url: String,
    pub git_url: String,
    pub issue_comment_url: String,
    pub issue_events_url: String,
    pub issues_url: String,
    pub keys_url: String,
    pub labels_url: String,
    pub languages_url: String,
    pub merges_url: String,
    pub milestones_url: String,
    pub notifications_url: String,
    pub pulls_url: String,
    pub releases_url: String,
    pub ssh_url: String,
    pub stargazers_url: String,
    pub statuses_url: String,
    pub subscribers_url: String,
    pub subscription_url: String,
    pub tags_url: String,
    pub teams_url: String,
    pub trees_url: String,
    pub clone_url: String,
    pub mirror_url: Option<String>,
    pub hooks_url: String,
    pub svn_url: String,
    pub homepage: Option<String>,
    pub language: Option<String>,
    pub forks_count: i64,
    pub stargazers_count: i64,
    pub watchers_count: i64,
    /// The size of the repository. Size is calculated hourly. When a repository is initially created, the size is 0.
    pub size: i64,
    pub default_branch: String,
    pub open_issues_count: i64,
    pub is_template: Option<bool>,
    pub topics: Option<Vec<String>>,
    pub has_issues: bool,
    pub has_projects: bool,
    pub has_wiki: bool,
    pub has_pages: bool,
    pub has_downloads: bool,
    pub has_discussions: Option<bool>,
    pub archived: bool,
    pub disabled: bool,
    pub visibility: String,
    pub pushed_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    // pub permissions: Option<Box<crate::models::RepositoryTemplateRepositoryPermissions>>,
    pub role_name: Option<String>,
    pub temp_clone_token: Option<String>,
    pub delete_branch_on_merge: Option<bool>,
    pub subscribers_count: Option<i64>,
    pub network_count: Option<i64>,
    // pub code_of_conduct: Option<Box<crate::models::CodeOfConduct>>,
    // pub license: Option<Option<Box<crate::models::MinimalRepositoryLicense>>>,
    pub forks: Option<i64>,
    pub open_issues: Option<i64>,
    pub watchers: Option<i64>,
    pub allow_forking: Option<bool>,
    pub web_commit_signoff_required: Option<bool>,
    // pub security_and_analysis: Option<Option<Box<crate::models::MinimalRepositorySecurityAndAnalysis>>>,
}

// -------------------------------------------------------------------------------------------------
// OrganizationShort
// Defined as in: https://docs.github.com/en/rest/orgs/orgs?apiVersion=2022-11-28#list-organizations
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct OrganizationShort {
    pub login: String,
    pub id: i64,
    pub node_id: String,
    pub url: String,
    pub repos_url: String,
    pub events_url: String,
    pub hooks_url: String,
    pub issues_url: String,
    pub members_url: String,
    pub public_members_url: String,
    pub avatar_url: String,
    pub description: Option<String>,
}
