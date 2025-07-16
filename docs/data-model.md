# Data Model

## The datastore
The _datastore_ is a special directory that Nosey Parker uses to record its findings and maintain its internal state.
A datastore will be implicitly created by the `scan` command if needed.

## Blobs
Each scanned input is called a _blob_.
Each blob has a unique blob ID, which is a SHA-1 digest computed the same way `git` does.

## Provenance
Each blob has one or more _provenance_ entries associated with it.
A provenance entry is metadata that describes how the input was discovered, such as a file on the filesystem or a file in Git repository history.

## Rules
Nosey Parker is a rule-based system that uses regular expressions.
Each _rule_ has a single pattern with at least one capture group that isolates the match content from the surrounding context.
You can list available rules with `noseyparker rules list`.

## Rulesets
A collection of rules is organized into a _ruleset_.
Nosey Parker's default ruleset includes rules that detect things that appear to be secrets.
Other rulesets are available; you can list them with `noseyparker rules list.`

## Matches
When a rule's pattern matches an input, it produces a _match_.
A match is uniquely defined by a rule, blob ID, start byte offset, and end byte offset; these fields are used to compute a unique match identifier.

## Findings
Matches that share a rule and capture groups are combined into a _finding_.
In other words, a _finding_ is a group of matches.
This is Nosey Parker's top-level unit of reporting.
