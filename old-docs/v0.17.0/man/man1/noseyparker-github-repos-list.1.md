# NAME

noseyparker-github-repos-list - List repositories belonging to a
specific user or organization

# SYNOPSIS

**noseyparker github repos list** \[**--user**\] \[**--organization**\]
\[**--all-organizations**\] \[**-o**\|**--output**\]
\[**-f**\|**--format**\] \[**--ignore-certs**\] \[**--github-api-url**\]
\[**-v**\|**--verbose**\]... \[**-q**\|**--quiet**\] \[**--color**\]
\[**--progress**\] \[**--rlimit-nofile**\] \[**--sqlite-cache-size**\]
\[**--enable-backtraces**\] \[**-h**\|**--help**\]

# DESCRIPTION

List repositories belonging to a specific user or organization

# OPTIONS

**--user**=*USER*  
Select repositories belonging to the specified user

This option can be repeated.

**--organization**=*ORGANIZATION*  
Select repositories belonging to the specified organization

This option can be repeated.

**--all-organizations**  
Select repositories belonging to all organizations

This only works with a GitHub Enterprise Server instance. The
\`--github-api-url\` option must be specified.

**-o**, **--output**=*PATH*  
Write output to the specified path

If this argument is not provided, stdout will be used.

**-f**, **--format**=*FORMAT* \[default: human\]  
Write output in the specified format  

  
*Possible values:*

- human: A text-based format designed for humans

- json: Pretty-printed JSON format

- jsonl: JSON Lines format

**--ignore-certs**  
Ignore validation of TLS certificates

**--github-api-url**=*URL* \[default: https://api.github.com/\]  
Use the specified URL for GitHub API access

If accessing a GitHub Enterprise Server instance, this value should be
the entire base URL include the \`api/v3\` portion, e.g.,
\`https://github.example.com/api/v3\`.

**-v**, **--verbose**  
Enable verbose output

This can be repeated up to 3 times to enable successively more output.

**-q**, **--quiet**  
Suppress non-error feedback messages

This silences WARNING, INFO, DEBUG, and TRACE messages and disables
progress bars. This overrides any provided verbosity and progress
reporting options.

**--color**=*MODE* \[default: auto\]  
Enable or disable colored output

When this is "auto", colors are enabled for stdout and stderr when they
are terminals.

If the \`NO_COLOR\` environment variable is set, it takes precedence and
is equivalent to \`--color=never\`.  

  
\[*possible values:* auto, never, always\]

**--progress**=*MODE* \[default: auto\]  
Enable or disable progress bars

When this is "auto", progress bars are enabled when stderr is a
terminal.  

  
\[*possible values:* auto, never, always\]

**--rlimit-nofile**=*LIMIT* \[default: 16384\]  
Set the rlimit for number of open files to LIMIT

This should not need to be changed from the default unless you run into
crashes from running out of file descriptors.

**--sqlite-cache-size**=*SIZE* \[default: -1048576\]  
Set the cache size for sqlite connections to SIZE

This has the effect of setting SQLites \`pragma cache_size=SIZE\`. The
default value is set to use a maximum of 1GiB for database cache. See
\<https://sqlite.org/pragma.html#pragma_cache_size\> for more details.

**--enable-backtraces**=*BOOL* \[default: true\]  
Enable or disable backtraces on panic

This has the effect of setting the \`RUST_BACKTRACE\` environment
variable to 1.  

  
\[*possible values:* true, false\]

**-h**, **--help**  
Print help (see a summary with -h)
