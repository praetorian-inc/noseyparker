//! Tests for Nosey Parker's `scan` command
use super::*;

mod appmaker;
mod basic;
mod copy_blobs;
mod git_url;
#[cfg(feature = "github")]
mod github;
mod snippet_length;
mod with_ignore;
