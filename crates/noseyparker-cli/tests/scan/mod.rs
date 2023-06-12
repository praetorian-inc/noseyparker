//! Tests for Nosey Parker's `scan` command
use super::*;

mod basic;
mod git_url;
mod with_ignore;
mod snippet_length;

// TODO: add test for scanning with `--github-user`
// TODO: add test for scanning with `--github-org`
// TODO: add test for caching behavior of rescanning `--git-url`
// TODO: add test for scanning multiple times with changing `--git-clone-mode` option
// TODO: add test for scanning with `--git-clone-mode bare` and `--git-clone-mode mirror`
// TODO: add test for scanning with `--github-api-url`
// TODO: add tests for SARIF output format
