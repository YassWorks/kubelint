# kubelint

[![CI](https://github.com/YassWorks/kubelint/actions/workflows/ci.yml/badge.svg)](https://github.com/YassWorks/kubelint/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/YassWorks/kubelint)](https://github.com/YassWorks/kubelint/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**A fast, friendly linter for Kubernetes manifests.** The Ruff/ESLint experience, for
Kubernetes YAML.

> **Status: early development.** Every merge to `main` that changes code is released
> automatically, so versions move fast and anything may change before 1.0. Follow
> progress in [`docs/roadmap.md`](docs/roadmap.md).

## Quickstart

Download the archive for your platform from the
[latest release](https://github.com/YassWorks/kubelint/releases/latest), unpack it and
put `kubelint` on your `PATH`. Linux binaries are static and run on any distribution.

Or build it from source (you need [rustup](https://rustup.rs); the pinned toolchain
installs itself):

```bash
cargo install --locked --git https://github.com/YassWorks/kubelint kubelint
```

Lint every Kubernetes YAML file under the current directory:

```bash
kubelint check .
```

kubelint searches directories for `*.yaml` and `*.yml` files, honoring `.gitignore`, and
ignores YAML that is not a Kubernetes object (CI files, Helm values). For a directory
`k8s/` with two broken files, the output looks like this:

```text
$ kubelint check k8s
error[KL1002]: object `web-config` has no `kind`
 --> k8s/config.yaml:1:1
  |
1 | apiVersion: v1
  | ^^^^^^^^^^^^^^
  |
  = help: add the object's type, such as `kind: Deployment`
  = note: rule `missing-kind`

error[KL1001]: `Deployment` object has no `apiVersion`
 --> k8s/web.yaml:1:1
  |
1 | kind: Deployment
  | ^^^^^^^^^^^^^^^^
  |
  = help: add `apiVersion: apps/v1`
  = note: rule `missing-api-version`

error[KL0001]: invalid YAML: while parsing a block collection, did not find expected '-' indicator
  --> k8s/web.yaml:14:6
   |
14 |      targetPort: 8080
   |      ^
   |
   = help: fix the YAML syntax; kubelint skips this document until it parses
   = note: rule `invalid-yaml`

Found 3 errors. Checked 2 files and 2 objects in 2ms.
```

A few more things worth knowing:

- **Exit codes:** `0` when nothing was found, `1` when there are findings, `2` when
  kubelint could not run (a missing file, bad arguments). Use it as a CI gate as is.
- **Standard input:** `-` reads a manifest from stdin, for generated YAML:
  `kustomize build overlays/prod | kubelint check -`.
- **Helm charts:** a chart's `templates/` are not valid YAML before rendering, so
  kubelint skips them (and `Chart.yaml`) with a note. Lint the rendered output instead:
  `helm template charts/web | kubelint check -`.
- **Colors:** `--color auto|always|never`. `auto` colors a terminal and honors
  `NO_COLOR`.
- **Configuration:** a `kubelint.toml` (or `.kubelint.toml`) in the current directory or
  a parent picks the rules and the fail threshold:

  ```toml
  [lint]
  select = ["correctness", "security"]  # categories, code prefixes, codes or names
  ignore = ["KL2004"]
  fail-on = "error"                     # error | warning | info

  [lint.severity]
  KL3003 = "error"

  [files]
  exclude = ["vendor/**"]               # globs, relative to this file's directory
  ```

  `--select`, `--ignore` and `--fail-on` replace the file's values for one run,
  `--config <path>` picks another file and `--isolated` ignores it.
- **Suppressions:** silence one finding with a comment naming the rule's code or name,
  and say why after `--`:

  ```yaml
  # kubelint-disable-next-line KL2001 -- the CNI plugin needs this
  privileged: true
  ```

  `# kubelint-disable-line <rules>` works at the end of a line and
  `# kubelint-disable-file <rules>` anywhere in a file. KL0001 cannot be suppressed, and a
  suppression naming an unknown rule is reported (KL0003), so typos do not go unnoticed.
- **Output formats:** `--format json` prints one JSON document for scripts and
  `--format sarif` feeds GitHub code scanning, `--format github` annotates pull requests
  in GitHub Actions; see
  [output formats](docs/user/output-formats.md).
- **Rules:** `kubelint rules` lists every rule (`--format json` for scripts) and
  `kubelint rule KL1001` (or `kubelint rule missing-api-version`) explains one.

Only a handful of rules exist so far; see the [rule catalog](docs/rules/README.md) for
what is implemented and what is planned.

## What it catches

- **Broken manifests:** missing fields, selectors that match nothing, invalid resource
  quantities, env values YAML reads as numbers or booleans, and (planned) full schema
  validation against real Kubernetes API versions, including removed APIs.
- **Security risks:** privileged containers, running as root, host namespaces and paths,
  dangerous capabilities, secrets in plain env vars.
- **Reliability problems:** missing resource limits and requests, unpinned image tags,
  and more.

See the full [rule catalog](docs/rules/README.md).

## Why kubelint

- **Instant:** a single static binary written in Rust. No cluster, no network, no runtime.
- **Zero config:** `kubelint check .` finds your Kubernetes YAML and skips everything else.
- **Precise:** every finding points at the exact line and column.
- **Explains itself:** every finding says what is wrong, why it matters and how to fix it.
- **Fits your workflow:** terminal, CI (SARIF, GitHub annotations, pre-commit) and,
  later, your editor.

## Documentation

- [Configuration](docs/user/configuration.md): config file, rule selection, flags,
  exclusions and suppressions.
- [Output formats](docs/user/output-formats.md): human, JSON, SARIF and GitHub output.
- [Vision](docs/vision.md): goals and non-goals.
- [Roadmap](docs/roadmap.md): what is being built, in order.
- [Architecture](docs/architecture.md): how it works inside.
- [Contributing](docs/CONTRIBUTING.md): build, test, add a rule.

## License

[MIT](LICENSE)
