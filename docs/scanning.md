# Scanning Inputs

!!! note "Note on Docker usage"

    When using the Docker image, replace `noseyparker` in the following commands with a Docker invocation that uses a mounted volume:

    ```shell
    docker run -v "$PWD":/scan ghcr.io/praetorian-inc/noseyparker:latest <ARGS>
    ```

    The Docker container runs with `/scan` as its working directory, so mounting `$PWD` at `/scan` in the container will make tab completion and relative paths in your command-line invocation work.


## Scan filesystem content, including local Git repos
![Screenshot showing Nosey Parker's workflow for scanning the filesystem for secrets](usage-examples/gifs/02-scan-git-history.gif)

Nosey Parker has native support for scanning files, directories, and the entire history of Git repositories.

For example, if you have a Git clone of [CPython](https://github.com/python/cpython) locally at `cpython.git`, you can scan it with the `scan` command.
Nosey Parker will create a new datastore at `cpython.np` and saves its findings there.
(The name `cpython.np` is innessential, and can be whatever you want.)
```
$ noseyparker scan -d cpython.np cpython.git
Scanned 19.19 GiB from 335,849 blobs in 17 seconds (1.11 GiB/s); 2,178/2,178 new matches

 Rule                            Findings   Matches   Accepted   Rejected   Mixed   Unlabeled
──────────────────────────────────────────────────────────────────────────────────────────────
 Generic API Key                        1         8          0          0       0           1
 Generic Password                       8     1,283          0          0       0           8
 Generic Username and Password          2        40          0          0       0           2
 HTTP Bearer Token                      1       108          0          0       0           1
 PEM-Encoded Private Key               61       151          0          0       0          61
 netrc Credentials                     27       588          0          0       0          27

Run the `report` command next to show finding details.
```

See `noseyparker help scan` for more details.

## Scan a Git repo from an HTTPS URL

For example, to scan the Nosey Parker repo itself:
```
noseyparker scan --datastore np.noseyparker --git-url https://github.com/praetorian-inc/noseyparker
```

See `noseyparker help scan` for more details.

## Scan Git repos of a GitHub user or organization

Use `--github-user=USER` or `--github-org=ORG`. For example, to scan accessible repositories belonging to the [`octocat`](https://github.com/octocat) user:
```
noseyparker scan --datastore np.noseyparker --github-user octocat
```

These input specifiers will use an optional GitHub token if available in the `NP_GITHUB_TOKEN` environment variable.
Providing an access token gives a higher API rate limit and may make additional repositories accessible to you.

See `noseyparker help scan` for more details.
