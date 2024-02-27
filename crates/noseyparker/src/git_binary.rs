use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use tracing::{debug, debug_span};

use crate::git_url::GitUrl;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git execution failed: {0}")]
    IOError(#[from] std::io::Error),

    #[error("git execution failed\ncode={}\nstdout=```\n{}```\nstderr=```\n{}```",
            .status,
            String::from_utf8_lossy(.stdout),
            String::from_utf8_lossy(.stderr))]
    GitError {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        status: ExitStatus,
    },
}

pub struct Git {
    credentials: Vec<String>,
    ignore_certs: bool,
}

impl Git {
    pub fn new(ignore_certs: bool) -> Self {
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

        Self {
            credentials,
            ignore_certs,
        }
    }

    fn git(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.env("GIT_CONFIG_GLOBAL", "/dev/null");
        cmd.env("GIT_CONFIG_NOSYSTEM", "1");
        cmd.env("GIT_CONFIG_SYSTEM", "/dev/null");
        if self.ignore_certs {
            cmd.env("GIT_SSL_NO_VERIFY", "1");
        }
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
        Self::new(false)
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
