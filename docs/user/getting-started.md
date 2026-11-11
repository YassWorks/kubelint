# Getting started

This guide installs kubelint, runs it on a project, and shows how to read and act on
what it reports. It takes about five minutes.

## Install

Every release on the [releases page](https://github.com/YassWorks/kubelint/releases)
has an archive for each platform:

| Platform | Archive |
| --- | --- |
| Linux, x86-64 | `kubelint-<version>-x86_64-unknown-linux-musl.tar.gz` |
| Linux, ARM64 | `kubelint-<version>-aarch64-unknown-linux-musl.tar.gz` |
| macOS, Intel | `kubelint-<version>-x86_64-apple-darwin.tar.gz` |
| macOS, Apple silicon | `kubelint-<version>-aarch64-apple-darwin.tar.gz` |
| Windows, x86-64 | `kubelint-<version>-x86_64-pc-windows-msvc.zip` |

Each archive holds one directory with the `kubelint` binary, the license and the
README. The Linux binaries are static and run on any distribution. `SHA256SUMS` lists
the checksum of every archive.

On Linux or macOS, set the version and platform, then download and unpack:

```bash
VERSION=v0.17.0
TARGET=x86_64-unknown-linux-musl
curl -fsSL "https://github.com/YassWorks/kubelint/releases/download/$VERSION/kubelint-$VERSION-$TARGET.tar.gz" \
  | tar -xz --strip-components=1 "kubelint-$VERSION-$TARGET/kubelint"
sudo mv kubelint /usr/local/bin/
```

Take the latest version from the
[latest release](https://github.com/YassWorks/kubelint/releases/latest). To build from
source instead, install [rustup](https://rustup.rs) and run:

```bash
cargo install --locked --git https://github.com/YassWorks/kubelint kubelint
```

Check that it works:

```bash
kubelint --version
```

## Lint a project

From the root of a repository:

```bash
kubelint check .
```

kubelint searches the directories you name for `*.yaml` and `*.yml` files, honoring
`.gitignore`. The rules run on Kubernetes objects only. Other YAML (CI files, Helm
values) is only checked for valid syntax. You can also name files, or pass `-` to read standard input.

## Read a finding

```text
$ kubelint check k8s
warning[KL3003]: image `nginx:latest` uses the `latest` tag
  --> k8s/web.yaml:14:16
   |
14 |         image: nginx:latest
   |                ^^^^^^^^^^^^
   |
   = help: pin a version tag, such as `nginx:1.2.3`, or a digest, such as `nginx@sha256:<digest>`
   = note: rule `unpinned-image-tag`

error[KL1009]: env var `PORT` has a value that YAML reads as a number, not a string
  --> k8s/web.yaml:17:18
   |
17 |           value: 8080
   |                  ^^^^
   |
   = help: quote the value: `value: "8080"`
   = note: rule `non-string-env-value`

Found 1 error and 1 warning. Checked 1 file and 1 object in 3ms.
```

Each finding has:

- a **severity** (`error`, `warning` or `info`) and the **rule code**, such as `KL1009`;
- a message saying what is wrong;
- the **file, line and column**, and the offending YAML with the exact part underlined;
- a **help** line saying how to fix it;
- the rule's **name**, which you can use instead of its code anywhere.

To learn why a rule exists and see an example and its fix, ask kubelint:

```bash
kubelint rule KL1009
```

The same text is in the [rule catalog](../rules/README.md), one page per rule.
`kubelint rules` lists every rule and whether it runs by default.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Nothing at or above the `fail-on` severity (`warning` by default) was found. |
| `1` | At least one finding at or above `fail-on`. |
| `2` | kubelint could not run: a missing file, a bad argument or an invalid configuration. |

## Helm charts and Kustomize

kubelint lints rendered YAML. It skips a Helm chart's `templates/` and `Chart.yaml`
with a note, because templates are not valid YAML before rendering. Render first and
pipe the result in:

```bash
helm template charts/web | kubelint check -
```

```bash
kustomize build overlays/prod | kubelint check -
```

## Next steps

- Too many findings at first? Make only errors fail the run (`kubelint check --fail-on
  error .`), or pick rule categories in a [configuration file](configuration.md).
- A finding you have decided to accept? [Suppress it](suppressions.md) with a comment
  or an annotation, and say why.
- Ready to gate pull requests? See [CI integration](ci-integration.md).
