"""
This script generates MkDocs-compatible Markdown pages for the built-in rules of
Nosey Parker.

Note that MkDocs uses python-markdown, which is *not* a CommonMark-compatible
Markdown implementation.
"""

import json
import html
import re
import subprocess

from io import StringIO
from textwrap import dedent

import mkdocs_gen_files


def natural_sort_key(s, _nsre=re.compile(r'(\d+)')):
    """
    Sorts strings in "natural" order, so that, for example, `foo-1` comes
    before `foo-10`.
    """
    return [int(text) if text.isdigit() else text.lower()
            for text in _nsre.split(s)]


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





# Load the rules
contents = json.loads(subprocess.check_output(
    ['cargo', 'run', '--', 'rules', 'list', '-fjson'],
    encoding='utf-8',
))

# Generate mkdocs-literate-nav SUMMARY.md to specify page ordering and titles,
# and generate a single markdown page for each rule.
with mkdocs_gen_files.open(f'rules/SUMMARY.md', 'wt') as summary:
    # exclude the summary file (used for mkdocs-literate-nav) from search
    print(dedent('''\
        ---
        search:
          exclude: true
        ---
        '''), file=summary)

    for rule in sorted(contents['rules'], key=lambda r: natural_sort_key(r['id'])):
        rule_name = rule['name']
        rule_id = rule['id']
        rule_fname = rule_id
        syntax = rule['syntax']

        # Add entry to mkdocs-literate-nav SUMMARY.md for navigation
        print(f'* [{rule_name} (`{rule_id}`)]({rule_fname}.md)', file=summary)

        # Generate individual rule page
        buf = StringIO()

        print(f'# {rule_name} (`{rule_id}`)', file=buf)
        print(file=buf)

        print(f'### Description', file=buf)
        print(syntax.get('description') or 'N/A', file=buf)
        print(file=buf)

        print(f'### Categories', file=buf)
        for cat in syntax.get('categories', []):
            print(f'- {cat}', file=buf)
        print(file=buf)

        print(f'### Examples', file=buf)
        for ex in syntax.get('examples', []):
            print(code_block(ex), file=buf)
        print(file=buf)

        print(f'### References', file=buf)
        for ref in syntax.get('references', []):
            print(f'- <{ref}>', file=buf)
        print(file=buf)

        print(f'### Pattern', file=buf)
        print(code_block(syntax['pattern']), file=buf)

        md = buf.getvalue()
        # print(f'<<<{md}>>>')
        with mkdocs_gen_files.open(f'rules/{rule_fname}.md', 'wt') as outfile:
            print(md, file=outfile)
