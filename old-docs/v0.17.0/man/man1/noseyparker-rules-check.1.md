# NAME

noseyparker-rules-check - Check rules for problems

# SYNOPSIS

**noseyparker rules check** \[**-W**\|**--warnings-as-errors**\]
\[**--rules**\] \[**--ruleset**\] \[**-v**\|**--verbose**\]...
\[**-q**\|**--quiet**\] \[**--color**\] \[**--progress**\]
\[**--rlimit-nofile**\] \[**--sqlite-cache-size**\]
\[**--enable-backtraces**\] \[**-h**\|**--help**\]

# DESCRIPTION

Check rules for problems

If errors are detected or if warnings are detected and
\`--warnings-as-errors\` is specified, the program will exit with a
nonzero exit code.

# OPTIONS

**-W**, **--warnings-as-errors**  
Treat warnings as errors

**--rules**=*PATH*  
Load additional rules and rulesets from the specified file or directory

The paths can be either files or directories. Directories are
recursively walked and all discovered YAML files of rules and rulesets
will be loaded.

This option can be repeated.

**--ruleset**=*ID* \[default: default\]  
Enable the ruleset with the specified ID

The ID must resolve to a built-in ruleset or to an additional ruleset
loaded with the \`--rules=PATH\` option.

The special \`all\` ID causes all loaded rules to be used.

This option can be repeated.

Specifying this option disables the default ruleset. If you want to use
a custom ruleset in addition to the default ruleset, specify this option
twice, e.g., \`--ruleset default --ruleset CUSTOM_ID\`.

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
