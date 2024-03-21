Hide
Sleep 1s
Type "noseyparker -q scan --git-url 'https://github.com/Plazmaz/leaky-repo' && clear" Enter
Sleep 1s
Show

Sleep 1s

Type "# View findings in JSON format with the `report --format json` command." Sleep 0.5s Enter
Type "# You can redirect the output to a file using the `-o FILE` option." Sleep 0.5s Enter
Type "# This example assumes you already have a datastore from scanning at `datastore.np`." Sleep 1s Enter

Type "noseyparker report --format json -o findings.json" Sleep 0.5s Enter
Sleep 1s

Type "# You can pretty-print JSON using `jq`." Enter
Type "jq -C . findings.json | less -RS" Enter
Sleep 5 Type "q"
