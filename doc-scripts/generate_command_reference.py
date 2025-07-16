"""
This script generates MkDocs-compatible Markdown pages for the command-line
help of Nosey Parker commands.

Note that MkDocs uses python-markdown, which is *not* a CommonMark-compatible
Markdown implementation.
"""

import re
import subprocess
import tempfile

from pathlib import Path
from io import StringIO
from textwrap import dedent

import mkdocs_gen_files


def code_block(s: str) -> str:
    """
    Wrap the given string as a Markdown code block.

    This is done using backticks. The number of backticks is chosen to ensure
    that it does not appear within the content of the given string.
    """
    if (m := re.search(r'`+', s)) is not None:
        delim = '`' * (len(m.group(0)) + 1)
    else:
        delim = '```'

    return f'{delim}\n{s}\n{delim}'




# Use `noseyparker generate manpages` to emit troff manpages;
# convert each of those into markdown using Pandoc.
# Generate an explicit SUMMARY.md listing the pages and titles, for the
# `mkdocs-literate-nav` plugin to pickup.
with (
    tempfile.TemporaryDirectory() as outdir,
    mkdocs_gen_files.open(f'commands/SUMMARY.md', 'wt') as summary,
):
    # exclude the summary file (used for mkdocs-literate-nav) from search
    print(dedent('''\
        ---
        search:
          exclude: true
        ---
        '''), file=summary)

    subprocess.check_call(['cargo', 'run', '--', 'generate', 'manpages', '-o', outdir])

    manpages = [f for f in Path(outdir).glob('*.1') if not f.name == 'noseyparker.1']
    manpages.sort()

    # Put index page first
    print('- [Overview](index.md)', file=summary)

    for fname in manpages:
        md = subprocess.check_output(['pandoc', '--standalone', '-fman', '-tgfm', fname], encoding='utf-8')
        # pandoc generates escaped less-than characters, which seem to always do the wrong thing in markdown renderers?
        md = md.replace(r'\<', '<')

        m = re.match(r'^noseyparker-(.*)\.1$', fname.name)
        assert m is not None
        md_fname = f'{m.group(1)}.md'
        md_title = m.group(1).replace('-', ' ')

        print(f'- [`{md_title}`]({md_fname})', file=summary)

        with mkdocs_gen_files.open(f'commands/{md_fname}', 'wt') as outfile:
            print(md, file=outfile)
