# Rule catalog

This table is the single place where rule codes are allocated. Before implementing a rule,
find it here. Before inventing a new rule, add a row here first (status `planned`) in the
same PR or an earlier one. Codes and names are permanent once a release ships them (see
[Codes and names](#codes-and-names)).

Per-rule documentation pages (`docs/rules/<CODE>.md`) will be generated from each rule
by the rule docs generator in the M3 roadmap. Until then, each rule's explanation is the
source of truth (`kubelint rule <CODE>` prints it).

**Columns.** *Default* is whether the rule is enabled with zero configuration. *Severity*
is the default severity. *Milestone* is the roadmap milestone that ships it.

## Codes and names

- Every rule has a **code**, `KL` and four digits, and a **name** in kebab-case that
  describes the problem (`privileged-container`, not `check-privileged`). Both work
  everywhere a rule can be named, and the output shows both.
- The first digit is the **category** (table below). Selecting `KL2` or `security`
  selects the category's default rules.
- Codes are allocated only in this file and never reused. A released rule is never
  renamed. A rule that must go is deprecated first: it keeps working and warns for at
  least one minor release, and its code stays reserved.
- A rule ships **enabled by default** only if it is high-signal on real-world
  manifests. Opinionated or context-dependent rules ship disabled.

**Why both:** users name rules in configuration, suppression comments and CI dashboards,
and those references must stay valid and readable across releases.

**Rejected:**

- Names only (ESLint, kube-linter). Readable, but long in suppression comments and hard
  to sort or group.
- Codes only (older Pylint). Compact, but opaque.

## Code ranges

| Range | Category | Meaning |
| --- | --- | --- |
| `KL0xxx` | `syntax` | The file is not valid YAML or not parseable as intended. |
| `KL1xxx` | `correctness` | The Kubernetes API server would reject the object, or it cannot work as written. `KL11xx` is reserved for schema validation (M4). |
| `KL2xxx` | `security` | Valid, but weakens the security of the cluster or workload. |
| `KL3xxx` | `reliability` | Valid and safe, but likely to cause outages or unpredictable behavior. |

## Syntax

| Code | Name | Default | Severity | Status | Milestone | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL0001 | `invalid-yaml` | on (cannot be disabled) | error | implemented | M1 | The file cannot be parsed as YAML. |
| KL0002 | `duplicate-key` | on | error | implemented | M2 | The same key appears twice in one mapping; Kubernetes rejects it or silently keeps the last one. |
| KL0003 | `invalid-suppression` | on | warning | implemented | M2 | A `# kubelint-disable-...` comment names an unknown rule, names KL0001 (which cannot be suppressed) or names no rule, so it silences nothing. |

## Correctness

| Code | Name | Default | Severity | Status | Milestone | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL1001 | `missing-api-version` | on | error | implemented | M1 | Object has no `apiVersion`, or an empty one. |
| KL1002 | `missing-kind` | on | error | implemented | M1 | Object has no `kind`, or an empty one. |
| KL1003 | `missing-object-name` | on | error | implemented | M2 | No `metadata.name` (and no `metadata.generateName`). |
| KL1004 | `duplicate-object` | on | error | implemented | M2 | The same kind/namespace/name is defined more than once across the linted files. |
| KL1005 | `selector-mismatch` | on | error | implemented | M2 | A workload's `spec.selector` does not match its pod template labels. |
| KL1006 | `duplicate-container-name` | on | error | implemented | M2 | Two containers (including init containers) in one pod share a name. |
| KL1007 | `invalid-resource-quantity` | on | error | implemented | M2 | A CPU/memory value like `512mb` or `0.5cores` that Kubernetes cannot parse. |
| KL1008 | `invalid-object-name` | on | error | implemented | M2 | `metadata.name` is not valid for its kind (DNS-1123 subdomain or label, DNS-1035 for Services), or `metadata.namespace` is not a valid namespace name. |
| KL1009 | `non-string-env-value` | on | error | implemented | M2 | An env var `value` that YAML reads as a number or boolean (`8080`, `yes`, `true`). |
| KL1010 | `invalid-label` | on | error | implemented | M2 | A label key or value that breaks Kubernetes' label syntax, or a label value that YAML reads as a number or boolean. |
| KL1011 | `duplicate-env-name` | on | warning | implemented | M2 | The same env var name is set twice in one container; only the last one wins. |
| KL1101 | `unknown-field` | on | error | planned | M4 | A field that does not exist in the schema, with a "did you mean" suggestion. |
| KL1102 | `invalid-field-type` | on | error | planned | M4 | A value of the wrong type (string where an integer is expected, and so on). |
| KL1103 | `missing-required-field` | on | error | planned | M4 | A field the schema requires is absent. |
| KL1104 | `removed-api-version` | on | error | planned | M4 | An `apiVersion` removed in the target Kubernetes version, with its replacement. |
| KL1105 | `deprecated-api-version` | on | warning | planned | M4 | An `apiVersion` deprecated in the target Kubernetes version. |
| KL1106 | `invalid-enum-value` | on | error | planned | M4 | A value outside a field's allowed set (for example `imagePullPolicy: Sometimes`). |

## Security

| Code | Name | Default | Severity | Status | Milestone | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL2001 | `privileged-container` | on | error | implemented | M2 | `securityContext.privileged: true`. |
| KL2002 | `privilege-escalation-allowed` | on | warning | implemented | M2 | `allowPrivilegeEscalation` is not set to `false`. |
| KL2003 | `run-as-root` | on | warning | implemented | M2 | The container may run as root (`runAsNonRoot` not true, or `runAsUser: 0`), honoring pod-level defaults. |
| KL2004 | `host-namespace` | on | warning | implemented | M2 | `hostNetwork`, `hostPID` or `hostIPC` is enabled. |
| KL2005 | `host-path-volume` | on | warning | implemented | M2 | A `hostPath` volume exposes the node's filesystem. |
| KL2006 | `dangerous-capability` | on | warning | implemented | M2 | Adds a Linux capability outside the `baseline` Pod Security Standard list, like `SYS_ADMIN`, `NET_ADMIN` or `ALL`. |
| KL2007 | `docker-socket-mount` | on | error | implemented | M2 | A `hostPath` volume mounts the container runtime socket (`docker.sock`, `containerd.sock`, `crio.sock`...). |
| KL2008 | `secret-in-env-literal` | on | warning | implemented | M2 | An env var whose name ends in a secret word (`PASSWORD`, `TOKEN`, `API_KEY`...) has a literal value instead of `valueFrom.secretKeyRef`. |
| KL2009 | `writable-root-filesystem` | off | warning | implemented | M2 | `readOnlyRootFilesystem` is not `true`. |

KL2006 calls a capability dangerous when the `baseline` Pod Security Standard forbids
adding it.

**Why:** the list is maintained upstream, enforced by Pod Security Admission, and a
manifest that passes KL2006 also passes that admission check.

**Rejected:** a hand-picked list of the worst capabilities (`SYS_ADMIN`, `NET_ADMIN`,
`ALL`). Shorter, but it would pass manifests the cluster rejects.

KL2008 looks only at the end of an env var name (`DB_PASSWORD`, `GITHUB_TOKEN`,
`AWS_SECRET_ACCESS_KEY`).

**Why:** a secret word in the middle usually names something about the secret, not
the secret itself (`TOKEN_URL`, `PASSWORD_FILE`, `AWS_ACCESS_KEY_ID`). Matching the end
keeps the rule low noise.

**Rejected:** matching a secret word anywhere in the name (reports file paths and
URLs); a list of known names (misses every name it does not list).

## Reliability

| Code | Name | Default | Severity | Status | Milestone | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL3001 | `missing-memory-limit` | on | warning | implemented | M2 | A container has no `resources.limits.memory`. |
| KL3002 | `missing-resource-requests` | on | warning | implemented | M2 | A container has no CPU or memory `requests`. |
| KL3003 | `unpinned-image-tag` | on | warning | implemented | M2 | An image uses `:latest` or no tag at all (digests are fine). |
| KL3004 | `missing-readiness-probe` | off | info | implemented | M2 | A long-running workload container has no `readinessProbe`. |
| KL3005 | `single-replica` | off | info | planned | M2 | A Deployment or StatefulSet runs exactly one replica. |

KL3004 checks the regular containers of Deployments, StatefulSets, DaemonSets,
ReplicaSets and ReplicationControllers only.

**Why:** these run long and are what Services route to. Jobs and CronJobs run to
completion, so readiness gates nothing. Nothing rolls out a bare Pod, and bare Pods are
rare in production. Sidecars are usually injected or third-party, so their authors, not
the manifest's, choose their probes. Kubernetes rejects `readinessProbe` on plain init
and ephemeral containers.

**Rejected:** checking bare Pods (noise from test and debug pods); checking sidecars
(findings the manifest author often cannot fix); checking Jobs and CronJobs.
