use secrecy::SecretString;

// -------------------------------------------------------------------------------------------------
// Auth
// -------------------------------------------------------------------------------------------------
/// Supported forms of authentication
pub enum Auth {
    /// No authentication
    Unauthenticated,

    /// Authenticate with a GitHub Personal Access Token
    PersonalAccessToken(SecretString),
}
