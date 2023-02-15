use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use tracing::{debug, debug_span};

#[derive(Debug)]
pub enum GitError
{
    IOError(std::io::Error),
    GitError {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        status: ExitStatus,
    },
}

impl From<std::io::Error> for GitError {
    fn from(err: std::io::Error) -> GitError {
        GitError::IOError(err)
    }
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::IOError(e) => write!(f, "{e}"),
            GitError::GitError {
                stdout: _,
                stderr: _,
                status,
            } => write!(f, "git execution failed: {}", status),
        }
    }
}

impl std::error::Error for GitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GitError::IOError(e) => Some(e),
            GitError::GitError {..} => None,
        }
    }
}

pub struct Git {
    credentials: Vec<String>,
}

impl Git {
    pub fn new() -> Self {
        let credentials: Vec<String> = // if std::env::var("NP_GITHUB_TOKEN").is_ok() {
            [
                "-c",
                r#"credential.helper="#,
                "-c",
                r#"credential.helper=!_ghcreds() { echo username="$NP_GITHUB_TOKEN"; echo password=; }; _ghcreds"#,
            ].iter().map(|s| s.to_string()).collect()
        // } else {
        //     vec![]
        // };
        ;

        Self { credentials }
    }

    fn git(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.env("GIT_CONFIG_GLOBAL", "/dev/null");
        cmd.env("GIT_CONFIG_NOSYSTEM", "1");
        cmd.env("GIT_CONFIG_SYSTEM", "/dev/null");
        cmd.args(&self.credentials);
        cmd.stdin(Stdio::null());
        cmd
    }

    pub fn update_mirrored_clone(&self, repo_url: &str, output_dir: &Path) -> Result<(), GitError> {
        let _span = debug_span!("git_update", "{repo_url} {}", output_dir.display()).entered();
        debug!("Attempting to update clone of {repo_url} at {}", output_dir.display());

        let mut cmd = self.git();
        cmd.arg("--git-dir")
            .arg(output_dir)
            .arg("remote")
            .arg("update")
            .arg("--prune");

        debug!("{cmd:#?}");
        let output = cmd.output()?;
        if !output.status.success() {
            return Err(GitError::GitError {
                stdout: output.stdout,
                stderr: output.stderr,
                status: output.status,
            });
        }
        Ok(())
    }

    pub fn create_fresh_mirrored_clone(&self, repo_url: &str, output_dir: &Path) -> Result<(), GitError> {
        let _span = debug_span!("git_clone", "{repo_url} {}", output_dir.display()).entered();
        debug!("Attempting to create fresh clone of {} at {}", repo_url, output_dir.display());

        let mut cmd = self.git();
        cmd.arg("clone")
            .arg("--mirror")
            .arg(repo_url)
            .arg(output_dir);

        debug!("{cmd:#?}");
        let output = cmd.output()?;
        if !output.status.success() {
            return Err(GitError::GitError {
                stdout: output.stdout,
                stderr: output.stderr,
                status: output.status,
            });
        }
        Ok(())
    }
}

impl Default for Git {
    /// Equivalent to `Git::new()`
    fn default() -> Self {
        Self::new()
    }
}
