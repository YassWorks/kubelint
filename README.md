# kubelint

**A fast, friendly linter for Kubernetes manifests.** The Ruff/ESLint experience, for
Kubernetes YAML.

> **Status: pre-alpha.** Nothing to install yet. The first release (v0.1) is planned for
> late November 2026. Follow progress in [`docs/roadmap.md`](docs/roadmap.md).

```text
$ kubelint check deploy.yaml
error[KL2001]: container "api" runs in privileged mode (privileged-container)
  --> deploy.yaml:24:23
   |
24 |             privileged: true
   |                         ^^^^ this gives the container full access to the node
   |
   = help: remove `privileged: true`, or grant only the capabilities it needs

warning[KL3001]: container "api" has no memory limit (missing-memory-limit)
  --> deploy.yaml:18:13
   |
18 |           - name: api
   |             ^^^^^^^^^ add `resources.limits.memory`
   |
   = help: without a limit, one pod can use all the memory on its node

Checked 14 files (31 objects) in 3ms. 1 error, 1 warning.
```

_(Planned output. This is the target the project is building toward.)_

## What it catches

- **Broken manifests:** missing fields, selectors that match nothing, invalid resource
  quantities, env values YAML reads as numbers or booleans, and (in v0.2) full schema
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

- [Vision](docs/vision.md): goals and non-goals.
- [Roadmap](docs/roadmap.md): what is being built, in order.
- [Architecture](docs/architecture.md): how it works inside.
- [Contributing](docs/CONTRIBUTING.md): build, test, add a rule.

## License

[MIT](LICENSE)
