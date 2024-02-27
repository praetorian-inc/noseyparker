use std::path::PathBuf;
use url::Url;

#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub struct GitUrl(Url);

impl GitUrl {
    /// Convert this URL into a path.
    /// This avoids potential path traversal issues with URLs like
    /// `https://example.com/../boom.git`.
    pub fn to_path_buf(&self) -> std::path::PathBuf {
        let mut result = PathBuf::new();
        result.push(self.0.scheme());

        let host_string = match self.0.host().expect("host should be non-empty") {
            url::Host::Domain(host) => host.to_owned(),
            url::Host::Ipv4(addr) => addr.to_string(),
            url::Host::Ipv6(addr) => addr.to_string(),
        };
        if let Some(port) = self.0.port() {
            result.push(format!("{host_string}:{port}"));
        } else {
            result.push(host_string);
        }
        result.extend(self.0.path_segments().expect("path segments should decode"));

        result
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for GitUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

const GIT_URL_ERROR_MESSAGE: &str =
    "only https URLs without credentials, query parameters, or fragment identifiers are supported";

impl std::str::FromStr for GitUrl {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Url::parse(s) {
            Err(_e) => Err(GIT_URL_ERROR_MESSAGE),
            Ok(url) => Self::try_from(url),
        }
    }
}

impl TryFrom<Url> for GitUrl {
    type Error = &'static str;

    fn try_from(url: Url) -> Result<Self, Self::Error> {
        if url.scheme() != "https" {
            return Err(GIT_URL_ERROR_MESSAGE);
        }

        if url.host().is_none() {
            return Err(GIT_URL_ERROR_MESSAGE);
        }

        if !url.username().is_empty() || url.password().is_some() {
            return Err(GIT_URL_ERROR_MESSAGE);
        }

        if url.query().is_some() {
            return Err(GIT_URL_ERROR_MESSAGE);
        }

        if url.fragment().is_some() {
            return Err(GIT_URL_ERROR_MESSAGE);
        }

        match url.path_segments() {
            None => return Err(GIT_URL_ERROR_MESSAGE),
            Some(segments) => {
                for segment in segments {
                    if segment == ".." {
                        return Err(GIT_URL_ERROR_MESSAGE);
                    }
                }
            }
        }

        Ok(GitUrl(url))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::Path;
    use std::str::FromStr;

    #[test]
    fn bad_scheme_01() {
        assert!(GitUrl::from_str("file://rel_repo.git").is_err());
    }

    #[test]
    fn bad_scheme_02() {
        assert!(GitUrl::from_str("file:///abs_repo.git").is_err());
    }

    #[test]
    fn bad_scheme_03() {
        assert!(GitUrl::from_str("ssh://example.com/repo.git").is_err());
    }

    #[test]
    fn bad_scheme_04() {
        assert!(GitUrl::from_str("http://example.com/repo.git").is_err());
    }

    #[test]
    fn bad_query_params() {
        assert!(GitUrl::from_str("https://example.com/repo.git?admin=1").is_err());
    }

    #[test]
    fn ok_empty_path_01() {
        assert_eq!(
            GitUrl::from_str("https://example.com")
                .unwrap()
                .to_path_buf(),
            Path::new("https/example.com")
        )
    }

    #[test]
    fn ok_empty_path_02() {
        assert_eq!(
            GitUrl::from_str("https://example.com/")
                .unwrap()
                .to_path_buf(),
            Path::new("https/example.com")
        )
    }

    #[test]
    fn ok_01() {
        assert_eq!(
            GitUrl::from_str("https://github.com/praetorian-inc/noseyparker.git")
                .unwrap()
                .to_path_buf(),
            Path::new("https/github.com/praetorian-inc/noseyparker.git")
        );
    }

    #[test]
    fn ok_relpath_01() {
        assert_eq!(
            GitUrl::from_str("https://example.com/../boom.git")
                .unwrap()
                .to_path_buf(),
            Path::new("https/example.com/boom.git")
        );
    }

    #[test]
    fn ok_relpath_02() {
        assert_eq!(
            GitUrl::from_str("https://example.com/root/../boom.git")
                .unwrap()
                .to_path_buf(),
            Path::new("https/example.com/boom.git")
        );
    }

    #[test]
    fn ok_relpath_03() {
        assert_eq!(
            GitUrl::from_str("https://example.com/root/..")
                .unwrap()
                .to_path_buf(),
            Path::new("https/example.com/")
        );
    }
}
