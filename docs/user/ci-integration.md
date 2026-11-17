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
VERSION=v0.18.15
TARGET=x86_64-unknown-linux-musl
curl -fsSL "https://github.com/YassWorks/kubelint/releases/download/$VERSION/kubelint-$VERSION-$TARGET.tar.gz" \
  | tar -xz --strip-components=1 "kubelint-$VERSION-$TARGET/kubelint"
./kubelint --version
```

For another platform, set `TARGET` to the part after `kubelint-` in its archive name,
listed in [getting started](getting-started.md#from-a-release-archive).

## GitHub Actions

The kubelint action installs a release and runs `kubelint check --format github`, so
each finding becomes an annotation on the changed line of the pull request, and in the
job log. No extra permission is needed.

```yaml
# .github/workflows/kubelint.yml
name: kubelint
on: [push, pull_request]
permissions:
  contents: read
jobs:
  kubelint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: YassWorks/kubelint@v0.18.7
```

The tag pins the kubelint version: `@v0.18.7` installs kubelint v0.18.7. The action
exists from v0.18.7 on and runs on Linux, macOS and Windows runners (x86-64, and ARM64
on Linux and macOS). Its inputs:

| Input | Default | Meaning |
| --- | --- | --- |
| `version` | the action's tag | The release to install, such as `v0.19.0`, or `latest`. With a ref that is not a release tag (a branch or a commit), the default is `latest`. |
| `format` | `github` | `github` for annotations, or `sarif` to write a SARIF file. |
| `args` | `.` | Options and paths for `kubelint check`, separated by spaces or newlines, for example `--fail-on error k8s`. |
| `sarif-file` | `kubelint.sarif` | Where `format: sarif` writes its output. |

The step fails when kubelint finds something at or above `fail-on`. To see findings in
the repository's Security tab instead, upload SARIF to code scanning, as shown in
[output formats](output-formats.md#uploading-to-github-code-scanning).

Without the action, download the release yourself as shown in
[installing in CI](#installing-in-ci) and run `./kubelint check --format github .`.

### Why a composite action

The action is a composite action: two shell scripts in `.github/scripts/` that download
the release archive, check it against the release's checksum file, and run it.

**Why:** it installs the same static binary as every other setup, starts in seconds and
works on every runner operating system.

**Rejected:**

- A Docker container action. It runs only on Linux runners and pulls an image on
  every run.
- A JavaScript action. It needs a Node build and a bundled `dist/` committed to the
  repository, for what two short scripts do.

## GitLab CI

```yaml
# .gitlab-ci.yml
kubelint:
  image: alpine:3
  variables:
    VERSION: v0.18.15
    TARGET: x86_64-unknown-linux-musl
  script:
    - apk add --no-cache curl
    - curl -fsSL "https://github.com/YassWorks/kubelint/releases/download/$VERSION/kubelint-$VERSION-$TARGET.tar.gz"
      | tar -xz --strip-components=1 "kubelint-$VERSION-$TARGET/kubelint"
    - ./kubelint check .
```

The Linux binary is static, so it runs on Alpine and any other image.

## Machine-readable output

`--format json` prints one JSON document with every finding, for scripts and other
tools. See [output formats](output-formats.md) for its fields.

## pre-commit

kubelint ships a [pre-commit](https://pre-commit.com) hook. It runs the `kubelint`
binary on your `PATH` on the YAML files of each commit, so install kubelint first (see
[getting started](getting-started.md#install)). Then add the hook, with `rev` set to
the release you installed. The hook exists from v0.18.6 on.

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/YassWorks/kubelint
    rev: v0.18.6
    hooks:
      - id: kubelint
```

The hook runs `kubelint check` on the staged `*.yaml` and `*.yml` files. YAML that is
not a Kubernetes object, such as CI files, is only checked for valid syntax. To pass
options, add them as `args`, for example `args: [--fail-on, error]`.

kubelint honors `[files] exclude` even for files named on the command line, so
pre-commit never lints excluded files.

### Why the hook needs kubelint installed

The hook uses `language: system`: it runs the `kubelint` already on your `PATH`, and
`rev` only pins the hook definition. pre-commit 4.4 renamed this language to
`unsupported` and still accepts `system` without a warning, so the hook keeps `system`
and works with older pre-commit versions too.

**Why:** it is the only kind of hook that works without a new package or a change to
kubelint's Cargo workspace, and you install the same static binary as everywhere else.

**Rejected:**

- `language: rust`. pre-commit runs `cargo install --path .` at the repository root,
  which fails on kubelint's virtual workspace manifest. It would also compile kubelint
  on every developer machine.
- A Python wheel on PyPI, so that pre-commit installs the binary itself. It needs a
  PyPI account, which the project does not have yet.
