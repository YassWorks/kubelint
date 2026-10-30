# Rule catalog

This table is the single place where rule codes are allocated. Before implementing a rule,
find it here. Before inventing a new rule, add a row here first (status `planned`) in the
same PR or an earlier one. Codes and names are permanent once a release ships them
([ADR 0004](../decisions/0004-rule-codes-and-categories.md)).

Per-rule documentation pages (`docs/rules/<CODE>.md`) will be generated from each rule's
`RuleMeta` once roadmap task M3-01 lands. Until then, the rule's Rust doc comment and
`explanation` field are the source of truth.

**Columns.** *Default* is whether the rule is enabled with zero configuration. *Severity*
is the default severity. *Task* is the roadmap task that implements it.

## Code ranges

| Range | Category | Meaning |
| --- | --- | --- |
| `KL0xxx` | `syntax` | The file is not valid YAML or not parseable as intended. |
| `KL1xxx` | `correctness` | The Kubernetes API server would reject the object, or it cannot work as written. `KL11xx` is reserved for schema validation (v0.2). |
| `KL2xxx` | `security` | Valid, but weakens the security of the cluster or workload. |
| `KL3xxx` | `reliability` | Valid and safe, but likely to cause outages or unpredictable behavior. |

## Syntax

| Code | Name | Default | Severity | Status | Task | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL0001 | `invalid-yaml` | on (cannot be disabled) | error | planned | M1-11 | The file cannot be parsed as YAML. |
| KL0002 | `duplicate-key` | on | error | planned | M2-01 | The same key appears twice in one mapping; Kubernetes rejects it or silently keeps the last one. |

## Correctness

| Code | Name | Default | Severity | Status | Task | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL1001 | `missing-api-version` | on | error | planned | M1-10 | Object has `kind` but no `apiVersion`. |
| KL1002 | `missing-kind` | on | error | planned | M1-10 | Object has `apiVersion` but no `kind`. |
| KL1003 | `missing-object-name` | on | error | planned | M2-11 | No `metadata.name` (and no `metadata.generateName`). |
| KL1004 | `duplicate-object` | on | error | planned | M2-12 | The same kind/namespace/name is defined more than once across the linted files. |
| KL1005 | `selector-mismatch` | on | error | planned | M2-13 | A workload's `spec.selector` does not match its pod template labels. |
| KL1006 | `duplicate-container-name` | on | error | planned | M2-14 | Two containers (including init containers) in one pod share a name. |
| KL1007 | `invalid-resource-quantity` | on | error | planned | M2-16 | A CPU/memory value like `512mb` or `0.5cores` that Kubernetes cannot parse. |
| KL1008 | `invalid-object-name` | on | error | planned | M2-18 | `metadata.name` is not a valid DNS-1123 name. |
| KL1009 | `non-string-env-value` | on | error | planned | M2-20 | An env var `value` that YAML reads as a number or boolean (`8080`, `yes`, `true`). |
| KL1010 | `invalid-label` | on | error | planned | M2-19 | A label key or value that breaks Kubernetes' label syntax. |
| KL1011 | `duplicate-env-name` | on | warning | planned | M2-21 | The same env var name is set twice in one container; only the last one wins. |
| KL1101 | `unknown-field` | on | error | planned | M4 | A field that does not exist in the schema, with a "did you mean" suggestion. |
| KL1102 | `invalid-field-type` | on | error | planned | M4 | A value of the wrong type (string where an integer is expected, and so on). |
| KL1103 | `missing-required-field` | on | error | planned | M4 | A field the schema requires is absent. |
| KL1104 | `removed-api-version` | on | error | planned | M4 | An `apiVersion` removed in the target Kubernetes version, with its replacement. |
| KL1105 | `deprecated-api-version` | on | warning | planned | M4 | An `apiVersion` deprecated in the target Kubernetes version. |
| KL1106 | `invalid-enum-value` | on | error | planned | M4 | A value outside a field's allowed set (for example `imagePullPolicy: Sometimes`). |

## Security

| Code | Name | Default | Severity | Status | Task | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL2001 | `privileged-container` | on | error | planned | M2-22 | `securityContext.privileged: true`. |
| KL2002 | `privilege-escalation-allowed` | on | warning | planned | M2-23 | `allowPrivilegeEscalation` is not set to `false`. |
| KL2003 | `run-as-root` | on | warning | planned | M2-24 | The container may run as root (`runAsNonRoot` not true, or `runAsUser: 0`), honoring pod-level defaults. |
| KL2004 | `host-namespace` | on | warning | planned | M2-25 | `hostNetwork`, `hostPID` or `hostIPC` is enabled. |
| KL2005 | `host-path-volume` | on | warning | planned | M2-26 | A `hostPath` volume exposes the node's filesystem. |
| KL2006 | `dangerous-capability` | on | warning | planned | M2-27 | Adds Linux capabilities like `SYS_ADMIN`, `NET_ADMIN` or `ALL`. |
| KL2007 | `docker-socket-mount` | on | error | planned | M2-28 | Mounts the container runtime socket (`docker.sock`, `containerd.sock`). |
| KL2008 | `secret-in-env-literal` | on | warning | planned | M2-29 | An env var whose name looks secret (`PASSWORD`, `TOKEN`, `API_KEY`...) has a literal value instead of `valueFrom.secretKeyRef`. |
| KL2009 | `writable-root-filesystem` | off | warning | planned | M2-30 | `readOnlyRootFilesystem` is not `true`. |

## Reliability

| Code | Name | Default | Severity | Status | Task | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL3001 | `missing-memory-limit` | on | warning | planned | M2-31 | A container has no `resources.limits.memory`. |
| KL3002 | `missing-resource-requests` | on | warning | planned | M2-32 | A container has no CPU or memory `requests`. |
| KL3003 | `unpinned-image-tag` | on | warning | planned | M2-33 | An image uses `:latest` or no tag at all (digests are fine). |
| KL3004 | `missing-readiness-probe` | off | info | planned | M2-34 | A long-running workload container has no `readinessProbe`. |
| KL3005 | `single-replica` | off | info | planned | M2-35 | A Deployment or StatefulSet runs exactly one replica. |
