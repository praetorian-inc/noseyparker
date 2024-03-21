# Usage Examples

## Overview
This directory contains scripts to produce GIFs of Nosey Parker usage examples.

These scripts are written using [`vhs`](https://github.com/charmbracelet/vhs), a console script runner and recording tool.
The example script fragments are in the [`examples`](examples) directory.
Each one is a `vhs` fragment, which is cobbled together with some common settings using the [`record-examples.zsh`](record-examples.zsh) script.

The generated GIFs are stored in the [`gifs`](gifs) directory.

## Generating GIFs
- Install `vhs`
- Install `noseyparker`
- `./record-examples.zsh`

## Errata: `vhs`
This tool, though it seems to be very popular, is full of significant bugs.
What I've seen in a day's usage:

- The `Show` and `Hide` commands seem to have a race condition, not reliably working without adding `Sleep 1s` calls before and after.
  Otherwise, you are likely to get the hidden commands included in the rendered output.

- The `Source` command is unsuitable for its main use case (putting common settings in a separate file); the settings seem to be applied, then partially forgotten.

- The framerate for GIF output is unreliable and doesn't correspond with `Sleep` durations, especially at higher framerates
