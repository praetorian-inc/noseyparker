# Nosey Parker Rules

At its core, Nosey Parker is a regular expression-based content matcher.
It uses a set of rules defined in YAML syntax to determine what matching content to report.

The default rules that Nosey Parker uses get embedded within the compiled `noseyparker` binary.
The source for these rules appears in the </crates/noseyparker/data/default/builtin/rules> directory.

## Rule structure
Nosey Parker's rules are written in YAML syntax.
A rules file contains a top-level YAML object with a `rules` field that is a list of rules.

Each rule is a YAML object, comprising a name, a regular expression, a list of references, a list of example inputs, and an optional list of non-example inputs.
It is easier to understand this from looking at sample rules.

The [`Jenkins Setup Admin Password`](/crates/noseyparker/data/default/builtin/rules/jenkins.yml) rule looks like this:
```
- name: Jenkins Setup Admin Password
  id: np.jenkins.2

  pattern: |
    (?x)(?m)
    Please\ use\ the\ following\ password\ to\ proceed\ to\ installation:
    (?: \n\n | \r\n\r\n )
    ([a-f0-9]{30,36})$

  examples:
  - |
      *************************************************************
      *************************************************************
      *************************************************************

      Jenkins initial setup is required. An admin user has been created and a password generated.
      Please use the following password to proceed to installation:

      bd9627decc6346d780b3b6ab6ea8fe1f

      This may also be found at: /root/.jenkins/secrets/initialAdminPassword

      *************************************************************
      *************************************************************
      *************************************************************

  categories: [fuzzy, secret]

  description: |
    A Jenkins setup wizard admin user password was detected.
    This password is used to configure a new Jenkins installation.
    An attacker with possession of this password could control the Jenkins instance.
    This could enable exfiltration of proprietary code, insertion of backdoors, or lateral movement to other resources.

  references:
  - https://www.jenkins.io/doc/book/installing/linux/#setup-wizard
```

The `name` field is intended primarily for human consumption, and is used heavily in various reports.
This can be any string.

The `id` field is a globally-unique identifier for the rule.
It must be a string of alphanumeric segments, optionally separated by hyphens or periods, and at most 20 characters long.

The `pattern` field in the rule the most essential part.
This is the regular expression pattern that controls what input gets matched.
Each pattern must have one or more capture groups that indicate where the secret or sensitive content is within the entire match.
In the case of this `Jenkins Setup Admin Password` rule, there is one capture group that is password, isolated from its surrounding context.
Other rules match things that are better specified.
For example, the [`GitHub Personal Access Token`](/crates/noseyparker/data/default/builtin/rules/github.yml) format includes a distinctive prefix that can be matched precisely.

The `description` field is a string designed for human consumption.
It is optional; if provided, it should indicate (a) what was detected, and (b) what an attacker could do with it.

The `references` field is a list of freeform strings.
In practice, these are URLs that describe the format of the thing being matched.

The `examples` field is a list of strings that are asserted to be matched by the rule.
This field is used for automated testing via `noseyparker rules check`.

The `negative_examples` field, if provided, is a list of strings that are asserted _not_ to be matched by the rule.
This field is also used for automated testing via `noseyparker rules check`.

The `categories` field is a list of freeform strings.
In practice, the following values are used:

- `secret`: the rule detects things that are in fact secrets
- `identifier`: the rule detects things that are not secrets but could be used to enumerate additional resources (e.g., S3 bucket names)
- `hashed`: the rule detects hashed payloads (e.g., bcrypt hashes)
- `test`: the rule detects test deployment-specific payloads (e.g., stripe test keys)
- `api`: the rule detects payloads used for API access
- `generic`: the rule is a "generic" one rather than one that detects a specific type of payload (e.g., username/password pairs)
- `fuzzy`: the rule pattern requires matching of non-payload surrounding context

## Pattern syntax
Nosey Parker uses a combination of regular expression engines in its implementation.
The pattern syntax that is accepted is (approximately) the intersection of Hyperscan and Rust `regex` crate syntax.

Specifically, the following constructs are supported:

- Literal characters
- Wildcard: `.`
- Alternation: `<PATTERN>|<PATTERN>`
- Repetition:
  - Zero or one: `?`
  - Zero or more: `*`
  - One or more: `+`
  - `i` and `j` inclusive: `{<i>,<j>}`
  - At least `i`: `{<i>,}`
  - Example `i`: `{<i>}`

- Character classes, ranges, and negated character classes, e.g., `[abc]`, `[a-zA-Z]`, `[^abc]`
- Specially named character classes:
  - Whitespace and non-whitespace: `\s` and `\S`
  - Word and non-word: `\w` and `\W`
  - Digit and non-digit: `\d` and `\D`

- Grouping: `(<PATTERN>)`
- Non-capturing grouping: `(?:<PATTERN>)`
- Word boundary and non-word-boundary zero-width assertions: `\b`, `\B`
- Start-of-input and end-of-input anchors: `^`, `$`

- Inline comments: `(?# <COMMENT>)`

- Inline flags:
  - The case-insensitive flag `(?i)`
  - The "extended syntax" flag `(?x)` (Helpful for making complicated patterns more legible!)
  - The "dotall" flag `(?s)`
  - The "multiline" flag `(?m)`

The following non-inclusive list of constructs are _not_ supported:

- Backreferences, e.g., `\1`

Note: Nosey Parker does regular expression matching over bytestrings, not over UTF-8 or otherwise encoded input.

For more reference:
- [The Hyperscan syntax reference](https://intel.github.io/hyperscan/dev-reference/compilation.html#pattern-support)
- [The Rust `regex` crate syntax reference](https://docs.rs/regex/latest/regex/index.html#syntax) and [caveats of matching bytestrings](https://docs.rs/regex/latest/regex/bytes/index.html#syntax)


## Guidelines for developing a new rule
If a rule identifies secret or sensitive content used by a well-known service and does so with high precision (i.e., few false positives), it is a candidate to be added to Nosey Parker's default rules.
Please open a pull request!

### Secret types with well-defined and distinct formats make for the best Nosey Parker rules
Some types of secrets have well-specified formats with distinctive prefixes or suffixes that are unlikely to appear accidentally.

For example, [AWS API keys](/crates/noseyparker/data/default/builtin/rules/aws.yml) start with one of a few 4-character prefixes followed by 16 hex digits.
A pattern that matches these needs no additional context.

Other types of secrets have a well-specified format but lack distinctiveness, such as [Sauce tokens](/crates/noseyparker/data/default/builtin/rules/sauce.yml), which appear to be simply version 4 UUIDs.
A pattern to match these requires looking at surrounding context, which is more likely to produce false positives.

### Include at least 1 capture group
Each rule pattern must include at least 1 capture group that isolates the content of the secret from the surrounding context.
Multiple captures groups are permitted; this can be useful for some types of secrets that involve multiple parts, such as a username and password.
For an example of this, see the [`netrc Credentials`](/crates/noseyparker/data/default/builtin/rules/netrc.yml) rule:
```
- name: netrc Credentials
  id: np.netrc.1

  pattern: |
    (?x)
    (?: (machine \s+ [^\s]+) | default)
    \s+
    login \s+ ([^\s]+)
    \s+
    password \s+ ([^\s]+)

  references:
  - https://everything.curl.dev/usingcurl/netrc
  - https://devcenter.heroku.com/articles/authentication#api-token-storage

  examples:
  - 'machine api.github.com login ziggy^stardust password 012345abcdef'
  - |
      ```
      machine raw.github.com
        login visionmedia
        password pass123
      ```

  - |
      """
      machine api.wandb.ai
        login user
        password 7cc938e45e63e9014f88f811be240ba0395c02dd
      """
```
This rule uses 3 capture groups to extract the optional machine name, the login name, and the password.

### Include a reference that describes what the rule matches
Each rule should include at least 1 reference, typically a URL.
This helps maintainers and operators better understand what a match might be.

### Strive for high-signal, low-noise rules, but keep patterns simple
Rules in Nosey Parker are selected to produce few false positives.
A rule's pattern should be precise as possible while minimizing its size.
It's always possible to expand a pattern to eliminate false positives, but doing so is usually a bad tradeoff in terms of comprehensibility.
For example, the [`Credentials in ODBC Connection String`](/crates/noseyparker/data/default/builtin/rules/odbc.yml) and [`LinkedIn Secret Key`](/crates/noseyparker/data/default/builtin/rules/linkedin.yml) rules are at the borderline of complexity we prefer to see.

### Make complex patterns comprehensible
Patterns comprehensibility decreases as patterns get longer.
A few tricks can ameliorate this:

1. Use YAML multiline scalars to avoid having to write escapes.
2. Use the "extended syntax" regular expression flag (`(?x)`) to let you split the pattern over multiple lines.
   (Note that you will need to explicitly escape whitespace in this mode if you want it to match.)
3. Use inline regular expression comments (`(?# COMMENT )`) judiciously.

The pattern in the [`JSON Web Token (base64url-encoded)`](/crates/noseyparker/data/default/builtin/rules/jwt.yml) rule demonstrates all these tricks:
```
# `header . payload . signature`, all base64-encoded
# Unencoded, the header and payload are JSON objects, usually starting with
# `{"`, which gets base64-encoded starting with `ey`.
pattern: |
  (?x)
  \b
  (
    ey[a-zA-Z0-9_-]+      (?# header )
    \.
    ey[a-zA-Z0-9_-]+      (?# payload )
    \.
    [a-zA-Z0-9_-]+        (?# signature )
  )
  (?:[^a-zA-Z0-9_-]|$)    (?# this instead of a \b anchor because that doesn't play nicely with `-` )
```

### Include real-looking examples found on the public internet
Each rule should include at least 1 example, preferably something that looks real that was found on the public internet.
The examples are used in automated testing via the `noseyparker rules check` command.

**Please invalidate any possible credentials in examples that you include by mangling the secret content in a structure-preserving way.**
We don't want to facilitate drive-by hacking by advertising real leaked secrets here.

### Test your rules to find problems
The `noseyparker rules check` command runs a number of checks over the builtin and ensures that the examples in the rule match (or not) as expected.
If you are writing non-default rules in separate YAML files, you can use `noseyparker rules check --rules-path=PATH` to check those files as well.

## Performance notes
Nosey Parker's implementation builds on top of the [Hyperscan](https://github.com/intel/hyperscan) library to efficiently match against all its rules simultaneously.
This is much faster than the naive approach of trying to match each regex rule in a loop on each input.
Adding more well-written rules to Nosey Parker should not significantly affect performance.
