# kubelint documentation

## Using kubelint

| Document | What it covers |
| --- | --- |
| [`user/getting-started.md`](user/getting-started.md) | Install kubelint, run it, read a finding, exit codes, Helm and Kustomize. |
| [`user/configuration.md`](user/configuration.md) | The config file, rule selection, command-line flags and exclusions. |
| [`user/suppressions.md`](user/suppressions.md) | Silencing findings with comments and the `kubelint/ignore` annotation. |
| [`user/ci-integration.md`](user/ci-integration.md) | Installing in CI, GitHub Actions, GitLab CI, pre-commit. |
| [`user/output-formats.md`](user/output-formats.md) | Human, JSON, SARIF and GitHub output, and the `rules` commands. |
| [`rules/`](rules/README.md) | The rule catalog: every rule code, planned or implemented. |

## Working on kubelint

| Document | What it covers |
| --- | --- |
| [`vision.md`](vision.md) | What kubelint is, who it is for, and what it will not do. |
| [`roadmap.md`](roadmap.md) | Milestones, what comes next, known gaps. |
| [`architecture.md`](architecture.md) | Crates, data flow, key types and design decisions. |
| [`conventions.md`](conventions.md) | How code, tests, docs, commits and PRs are written. |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Setup, local checks, and how to add a rule. |
| [`agent/playbook.md`](agent/playbook.md) | How the daily Claude Code routine that builds kubelint works. |
