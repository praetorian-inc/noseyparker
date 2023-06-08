use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use tracing::{debug, debug_span};

use crate::git_url::GitUrl;

#[derive(Debug)]
pub enum GitError {
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
            GitError::IOError(e) => write!(f, "git execution failed: {e}"),
            GitError::GitError {
                stdout,
                stderr,
                status,
            } => write!(
                f,
                "git execution failed\ncode={status}\nstdout=```\n{}```\nstderr=```\n{}```",
                String::from_utf8_lossy(stdout),
                String::from_utf8_lossy(stderr)
            ),
        }
    }
}

impl std::error::Error for GitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GitError::IOError(e) => Some(e),
            GitError::GitError { .. } => None,
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

    pub fn update_clone(&self, repo_url: &GitUrl, output_dir: &Path) -> Result<(), GitError> {
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

    pub fn create_fresh_clone(
        &self,
        repo_url: &GitUrl,
        output_dir: &Path,
        clone_mode: CloneMode,
    ) -> Result<(), GitError> {
        let _span = debug_span!("git_clone", "{repo_url} {}", output_dir.display()).entered();
        debug!("Attempting to create fresh clone of {} at {}", repo_url, output_dir.display());

        let mut cmd = self.git();
        cmd.arg("clone")
            .arg(clone_mode.arg())
            .arg(repo_url.as_str())
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

/// Represents the behavior for cloning a repository
#[derive(Debug, Clone, Copy)]
pub enum CloneMode {
    /// `--bare`
    Bare,

    /// `--mirror`
    Mirror,
}

impl CloneMode {
    pub fn arg(&self) -> &str {
        match self {
            Self::Bare => "--bare",
            Self::Mirror => "--mirror",
        }
    }
}
