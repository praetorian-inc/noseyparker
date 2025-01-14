# Reporting

## Report findings in human-readable text format
![Screenshot showing Nosey Parker's workflow for rendering its findings in human-readable format](usage-examples/gifs/03-report-human.gif)


## Report findings in JSON format
![Screenshot showing Nosey Parker's workflow for rendering its findings in JSON format](usage-examples/gifs/04-report-json.gif)


## Summarize findings

Nosey Parker prints out a summary of its findings when it finishes scanning.
You can also run this step separately after scanning:
```
$ noseyparker summarize --datastore np.cpython

 Rule                      Distinct Groups   Total Matches
───────────────────────────────────────────────────────────
 PEM-Encoded Private Key             1,076           1,192
 Generic Secret                        331             478
 netrc Credentials                      42           3,201
 Generic API Key                         2              31
 md5crypt Hash                           1               2
```

Additional output formats are supported, including JSON and JSON lines, via the `--format=FORMAT` option.

See `noseyparker help summarize` for more details.
