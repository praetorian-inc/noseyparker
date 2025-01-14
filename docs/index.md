---
hide: [toc, navigation]
---

# Nosey Parker: Find secrets in textual data

## Overview

Nosey Parker is a CLI tool that finds secrets and sensitive information in textual data.
It is essentially a special-purpose `grep`-like tool for detection of secrets.

It has been designed for offensive security (e.g., enabling lateral movement on red teams), but it can also be useful for defensive security testing.
It has found secrets in hundreds of offensive security engagements at [Praetorian](https://praetorian.com).

**Key features:**

- **Flexiblity:** It natively scans files, directories, GitHub, and Git history, and has an extensible input enumeration mechanism
- **Field-tested rules:** It uses regular expressions with [168 patterns](crates/noseyparker/data/default/builtin/rules) chosen for high precision based on feedback from security engineers
- **Signal-to-noise:** It deduplicates matches that share the same secret, reducing review burden by 10-1000x or more
- **Speed & scalability:** it can scan at GB/s on a multicore system, and has scanned inputs as large as 20TB during security engagements

The typical workflow is three phases:

1. Scan inputs of interest using the `scan` command
2. Report details of scan results using the `report` command
3. Review and triage findings

## License

Nosey Parker is licensed under the [Apache License, Version 2.0](LICENSE).

Any contribution intentionally submitted for inclusion in Nosey Parker by you, as defined in the Apache 2.0 license, shall be licensed as above, without any additional terms or conditions.

Nosey Parker also includes vendored copies of several other packages released under the Apache License and other permissive licenses; see [`LICENSE`](LICENSE) for details.


## Contributing

Ask questions or share ideas in the [Discussions](https://github.com/praetorian-inc/noseyparker/discussions) area.

Contributions are welcome, especially new regex rules.
Developing new regex rules is detailed in a [separate document](adding-rules.md).

If you are considering making significant code changes, please [open an issue](https://github.com/praetorian-inc/noseyparker/issues/new) or [start a discussion](https://github.com/praetorian-inc/noseyparker/discussions/new/choose) first.

This project has a number of [pre-commit](https://pre-commit.com/) hooks enabled that you are encouraged to use.
To install them in your local repo, make sure you have `pre-commit` installed and run:
```
$ pre-commit install
```
These checks will help to quickly detect simple errors.


## Integrations

Nosey Parker has a few third-party integrations:

- Nosey Parker is packaged in [Homebrew](https://formulae.brew.sh/formula/noseyparker)
- Nosey Parker is packaged in [Arch Linux](https://aur.archlinux.org/packages/noseyparker)
- A [GitHub Action](https://github.com/bpsizemore/noseyparker-action) that runs Nosey Parker is available
- [DefectDojo](https://defectdojo.org) includes a [parser for Nosey Parker v0.16 JSON](https://github.com/DefectDojo/django-DefectDojo/blob/c182e9ca9d8f981c15de2018f948fe69c4d1a800/docs/content/en/integrations/parsers/file/noseyparker.md)
- [Nemesis](https://github.com/SpecterOps/Nemesis) includes support for Nosey Parker

If you have an integration you'd like to share that's not listed here, please create a PR.
