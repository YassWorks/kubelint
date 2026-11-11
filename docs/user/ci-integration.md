# CI integration

kubelint is a single binary with meaningful exit codes, so any CI system can run it.
This page shows how to install it in CI, how to choose what fails the build, and
ready-made setups for GitHub Actions, GitLab CI and pre-commit.

## The basics

- **Exit codes:** `0` when nothing at or above `fail-on` was found, `1` when something
  was, `2` when kubelint could not run. A plain `kubelint check .` step is a CI gate.
- **What fails the build:** `fail-on` (`error`, `warning` or `info`, default `warning`)
  in the [configuration file](configuration.md), or `--fail-on` for one run. Findings
  below it are still printed.
- **Pin the version.** kubelint releases often and anything may change before 1.0.
  Download a fixed version so a new release never turns a green pipeline red by
  itself.
- **Rendered YAML only.** Lint the output of `helm template` or `kustomize build` with
  `kubelint check -` (see [getting started](getting-started.md#helm-charts-and-kustomize)).

## Installing in CI

Download the release archive for the runner's platform. On a Linux x86-64 runner:

```bash
VERSION=v0.17.0
TARGET=x86_64-unknown-linux-musl
curl -fsSL "https://github.com/YassWorks/kubelint/releases/download/$VERSION/kubelint-$VERSION-$TARGET.tar.gz" \
  | tar -xz --strip-components=1 "kubelint-$VERSION-$TARGET/kubelint"
./kubelint --version
```

The archive names for other platforms are listed in
[getting started](getting-started.md#install).

## GitHub Actions

With `--format github`, each finding becomes an annotation on the changed line of the
pull request, and in the job log. No extra permission is needed.

```yaml
# .github/workflows/kubelint.yml
name: kubelint
on: [push, pull_request]
permissions:
  contents: read
jobs:
  kubelint:
    runs-on: ubuntu-latest
    env:
      VERSION: v0.17.0
      TARGET: x86_64-unknown-linux-musl
    steps:
      - uses: actions/checkout@v4
      - name: Install kubelint
        run: |
          curl -fsSL "https://github.com/YassWorks/kubelint/releases/download/$VERSION/kubelint-$VERSION-$TARGET.tar.gz" \
            | tar -xz --strip-components=1 "kubelint-$VERSION-$TARGET/kubelint"
      - name: Lint manifests
        run: ./kubelint check --format github .
```

To see findings in the repository's Security tab instead, upload SARIF to code
scanning, as shown in [output formats](output-formats.md#uploading-to-github-code-scanning).
A ready-made GitHub Action is planned.

## GitLab CI

```yaml
# .gitlab-ci.yml
kubelint:
  image: alpine:3
  variables:
    VERSION: v0.17.0
    TARGET: x86_64-unknown-linux-musl
  script:
    - apk add --no-cache curl tar
    - curl -fsSL "https://github.com/YassWorks/kubelint/releases/download/$VERSION/kubelint-$VERSION-$TARGET.tar.gz"
      | tar -xz --strip-components=1 "kubelint-$VERSION-$TARGET/kubelint"
    - ./kubelint check .
```

The Linux binary is static, so it runs on Alpine and any other image. GNU `tar` is
installed because BusyBox `tar` does not take `--strip-components` with a member name.

## Machine-readable output

`--format json` prints one JSON document with every finding, for scripts and other
tools. See [output formats](output-formats.md) for its fields.

## pre-commit

A pre-commit hook is planned (see the [roadmap](../roadmap.md)). Until then, a local
hook can run an installed `kubelint` on the YAML files of a commit:

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: kubelint
        name: kubelint
        entry: kubelint check
        language: system
        types: [yaml]
```

kubelint honors `[files] exclude` even for files named on the command line, so
pre-commit never lints excluded files.
