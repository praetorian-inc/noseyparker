# NAME

noseyparker - Nosey Parker is a command-line program that finds secrets
and sensitive information in textual data and Git history.

# SYNOPSIS

**noseyparker** \[**-v**\|**--verbose**\]... \[**-q**\|**--quiet**\]
\[**--color**\] \[**--progress**\] \[**--rlimit-nofile**\]
\[**--sqlite-cache-size**\] \[**--enable-backtraces**\]
\[**-h**\|**--help**\] \[**-V**\|**--version**\] \<*subcommands*\>

# DESCRIPTION

Nosey Parker is a command-line program that finds secrets and sensitive
information in textual data and Git history.

# OPTIONS

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

**-V**, **--version**  
Print version

# SUBCOMMANDS

noseyparker-scan(1)  
Scan content for secrets

noseyparker-summarize(1)  
Summarize scan findings

noseyparker-report(1)  
Report detailed scan findings

noseyparker-github(1)  
Interact with GitHub

noseyparker-datastore(1)  
Manage datastores

noseyparker-rules(1)  
Manage rules

noseyparker-generate(1)  
Generate Nosey Parker release assets

# VERSION

v0.17.0

Build Configuration:

Build Timestamp: 2024-03-05T15:01:18.889024000Z

Commit Timestamp: 2024-03-05T09:55:01.000000000-05:00 Commit Branch:
HEAD Commit SHA: 41f30e2ca0186435ed3649e220549d4a4516109f

Cargo Features: disable_trace,log,release Debug: false Optimization: 3
Target Triple: x86_64-apple-darwin

Build System:

OS: Darwin OS Version: MacOS 13.6.4

CPU Vendor: GenuineIntel CPU Brand: Intel(R) Core(TM) i7-8700B CPU @
3.20GHz CPU Cores: 4

rustc Version: 1.76.0 rustc Channel: stable rustc Host Triple:
x86_64-apple-darwin rustc Commit Date: 2024-02-04 rustc Commit SHA:
07dca489ac2d933c78d3c5158e3f43beefeb02ce rustc LLVM Version: 17.0

# AUTHORS

Brad Larsen \<bradford.larsen@praetorian.com\>
