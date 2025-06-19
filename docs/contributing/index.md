# Contributing Overview

Ask questions or share ideas in the [Discussions](https://github.com/praetorian-inc/noseyparker/discussions) area.

Contributions are welcome, especially new regex rules.
See the guide on [Adding Rules](../adding-rules.md) for more details.

If you are considering making significant code changes, please [open an issue](https://github.com/praetorian-inc/noseyparker/issues/new) or [start a discussion](https://github.com/praetorian-inc/noseyparker/discussions/new/choose) first.
This will maximize the chance of your contribution being accepted.


## Pre-commit hooks

This project has a number of [pre-commit](https://pre-commit.com/) hooks enabled that you are encouraged to use.
To install them in your local repo, make sure you have `pre-commit` installed and run:

```
pre-commit install
```

These checks will help to quickly detect simple errors.


## Testing

Nosey Parker uses [Insta](https://insta.rs) snapshot-based testing for many of its tests.
If you have `cargo-insta` installed, you can run the following to run the test suites and retrain snapshots if appropriate:

```
cargo insta test --review
```
