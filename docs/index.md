---
hide: [toc, navigation]
---

# Nosey Parker: Find secrets in textual data

## Overview

Nosey Parker is a CLI tool that finds secrets and sensitive information in textual data.
It is essentially a special-purpose `grep`-like tool for detection of secrets.

It has been designed for offensive security (e.g., enabling lateral movement on red teams), but it can also be useful for defensive security testing.
It has found secrets in hundreds of offensive security engagements at [Praetorian](https://praetorian.com).


## Key features:

- **Flexiblity:** It natively scans files, directories, GitHub, and Git history, and can be extended [with arbitrary enumerator programs](scanning.md#scan-from-parquet-files-using-the-extensible-enumerator-mechanism)
- **Field-tested rules:** It uses regular expressions with [168 patterns](https://github.com/praetorian-inc/noseyparker/blob/main/crates/noseyparker/data/default/builtin/rules) chosen for high precision based on feedback from security engineers
- **Signal-to-noise:** It deduplicates matches that share the same secret, reducing review burden by 10-1000x or more
- **Speed & scalability:** it can scan at gigabytes per second, and has scanned inputs as large as 20TB during security engagements


## Contributing

Contributions are welcome, especially new regex rules.
See the dedicated [Contributing](contributing/index.md) guide for more detail.


## License

Nosey Parker is licensed under the Apache License, Version 2.0.

Any contribution intentionally submitted for inclusion in Nosey Parker by you, as defined in the Apache 2.0 license, shall be licensed as above, without any additional terms or conditions.

Nosey Parker also includes vendored copies of several other packages released under the Apache License and other permissive licenses; see [`LICENSE`](https://github.com/praetorian-inc/noseyparker/blob/main/LICENSE) for details.
