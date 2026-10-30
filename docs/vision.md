# Vision

> Status: locked on 2026-10-30 with the owner (YassWorks). Changing anything in the
> "Pillars" or "Non-goals" sections requires the owner's approval via
> [`agent/owner-inbox.md`](agent/owner-inbox.md).

## One sentence

**kubelint is a fast, friendly linter for Kubernetes manifests: the Ruff/ESLint
experience, for Kubernetes YAML.**

## The problem

Kubernetes manifests are long YAML files that are easy to get wrong:

- **Broken manifests.** A typo in a field name, a number where a string is required, a
  selector that does not match its pod labels. The cluster rejects the file, or worse,
  accepts it and does something you did not intend.
- **Security risks.** Containers running as root, privileged mode, host filesystem
  access. Valid YAML, but dangerous.
- **Reliability problems.** No memory limit, no readiness probe, images tagged
  `:latest`. Valid and safe, until production traffic arrives.

Existing tools each cover part of this, are slow to set up, or report problems in ways
that are hard to act on ("check failed" with no line number and no fix).

## What "fast and friendly" means

These are product requirements, not slogans. Every feature is judged against them.

1. **Instant.** A single static binary. Linting a large repo (10,000 objects) takes
   well under a second. No cluster, no network, no runtime to install.
2. **Zero config.** `kubelint check .` does the right thing in any repository. It finds
   Kubernetes YAML, skips YAML that is not Kubernetes (CI files, Helm templates) and
   enables a sensible default rule set.
3. **Precise.** Every diagnostic points at the exact line and column, with a rustc-style
   snippet of the offending YAML.
4. **Explains itself.** Every diagnostic says what is wrong, why it matters, and how to
   fix it. Every rule has a documentation page and `kubelint rule <code>` prints it.
5. **Low noise.** A rule that is often wrong is worse than no rule. Rules that are only
   sometimes appropriate ship disabled by default.
6. **Fits existing workflows.** Works in the terminal, in CI (exit codes, SARIF, GitHub
   annotations) and, later, live in the editor.

## Pillars (locked)

| Pillar | Decision |
| --- | --- |
| Identity | Fast and friendly. Great diagnostics first, breadth of rules second. |
| What it catches | Broken manifests (structure, then full schema validation), security risks, reliability best practices. Removed/deprecated API versions come with schema validation. |
| Where it runs | CLI first. Then CI integration (SARIF, GitHub Action, pre-commit). Then editor support (language server). |
| First release | v0.1, small and solid, around 3 weeks after kickoff (target: late November 2026). |
| Language | Rust. Single static binary. |
| License | MIT. |
| Development model | Built mostly by an autonomous daily Claude Code routine, merging its own small PRs on green CI, with all memory kept in this repository. See [`decisions/0008-autonomous-workflow.md`](decisions/0008-autonomous-workflow.md). |

## Release milestones

The detailed, task-level plan lives in [`roadmap.md`](roadmap.md).

- **v0.1, "useful in CI":** the CLI, about 20 high-signal rules (correctness, security,
  reliability), a config file, suppression comments, human/JSON/SARIF/GitHub output,
  installable from GitHub Releases and crates.io.
- **v0.2, "catches everything the API server would reject":** schema validation against
  real Kubernetes API schemas for a chosen Kubernetes version, including unknown fields
  with "did you mean" suggestions and removed API versions.
- **v0.3, "in your editor":** autofix (`--fix`) and a language server so diagnostics and
  fixes appear while typing (VS Code first).

## Non-goals (for now)

These were explicitly left out of scope by the owner. Do not build them without approval.

- **Helm and Kustomize as inputs.** kubelint lints rendered YAML. Users can pipe
  `helm template` or `kustomize build` output into `kubelint check -`. kubelint must
  detect Helm chart templates and skip them with a helpful note instead of reporting
  YAML errors.
- **Browser playground / WebAssembly build.**
- **User-written custom rules or a policy language.** The rule engine should not make
  this impossible later, but it is not a v0.x feature.
- **Talking to a live cluster.** kubelint is a static tool.
- **Admission controller / in-cluster mode.**

## Who it is for

1. **Application developers** who write manifests and want fast, clear feedback.
2. **Platform and DevOps engineers** who want a CI gate that is easy to adopt.
3. **People learning Kubernetes.** The explanations make the tool a teacher, not just a
   gate.

## Success looks like

- Someone runs `kubelint check .` on a real repository, fixes three real problems in five
  minutes, and adds it to CI the same day.
- A false positive is rare enough that people report it as a bug.
- Contributors can add a new rule in under an hour by following
  [`CONTRIBUTING.md`](CONTRIBUTING.md).
