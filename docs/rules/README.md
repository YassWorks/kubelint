# Rule catalog

This table is the single place where rule codes are allocated. Before implementing a rule,
find it here. Before inventing a new rule, add a row here first (status `planned`) in the
same PR or an earlier one. Codes and names are permanent once a release ships them (see
[Codes and names](#codes-and-names)).

Each rule has a documentation page, `docs/rules/<CODE>.md` (for example
[`KL2001.md`](KL2001.md)). The pages are generated from the rules with
`cargo xtask gen-rule-docs` and hold the same text as `kubelint rule <CODE>`. Edit the
rule's `.md` explanation, never the page.

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
| KL0002 | `duplicate-key` | on | error | implemented | M2 | The same key appears twice in one mapping; `kubectl` keeps the last one without a warning. |
| KL0003 | `invalid-suppression` | on | warning | implemented | M2 | A `# kubelint-disable-...` comment or a `kubelint/ignore` annotation names an unknown rule, names KL0001 (which cannot be suppressed) or names no rule, so it silences nothing. |

## Correctness

| Code | Name | Default | Severity | Status | Milestone | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL1001 | `missing-api-version` | on | error | implemented | M1 | Object has no `apiVersion`, or an empty one. |
| KL1002 | `missing-kind` | on | error | implemented | M1 | Object has no `kind`, or an empty one. |
| KL1003 | `missing-object-name` | on | error | implemented | M2 | No `metadata.name` (and no `metadata.generateName`). |
| KL1004 | `duplicate-object` | on | error | implemented | M2 | The same kind/namespace/name is defined more than once in one file. |
| KL1005 | `selector-mismatch` | on | error | implemented | M2 | A workload's `spec.selector` does not match its pod template labels. |
| KL1006 | `duplicate-container-name` | on | error | implemented | M2 | Two containers (including init containers) in one pod share a name. |
| KL1007 | `invalid-resource-quantity` | on | error | implemented | M2 | A CPU/memory value like `512mb` or `0.5cores` that Kubernetes cannot parse. |
| KL1008 | `invalid-object-name` | on | error | implemented | M2 | `metadata.name` or `metadata.generateName` is not valid for its kind (DNS-1123 subdomain or label, DNS-1035 for Services, path segment for RBAC and PodDisruptionBudget, with the CronJob and Job length limits), or `metadata.namespace` is not a valid namespace name. |
| KL1009 | `non-string-env-value` | on | error | implemented | M2 | An env var `value` that YAML reads as a number or boolean (`8080`, `yes`, `true`). |
| KL1010 | `invalid-label` | on | error | implemented | M2 | A label key or value that breaks Kubernetes' label syntax, or a label value that YAML reads as a number or boolean. |
| KL1011 | `duplicate-env-name` | on | warning | implemented | M2 | The same env var name is set twice in one container; only the last one wins. |
| KL1101 | `unknown-field` | on | error | planned | M4 | A field that does not exist in the schema, with a "did you mean" suggestion. |
| KL1102 | `invalid-field-type` | on | error | planned | M4 | A value of the wrong type (string where an integer is expected, and so on). |
| KL1103 | `missing-required-field` | on | error | planned | M4 | A field the schema requires is absent. |
| KL1104 | `removed-api-version` | on | error | planned | M4 | An `apiVersion` removed in the target Kubernetes version, with its replacement. |
| KL1105 | `deprecated-api-version` | on | warning | planned | M4 | An `apiVersion` deprecated in the target Kubernetes version. |
| KL1106 | `invalid-enum-value` | on | error | planned | M4 | A value outside a field's allowed set (for example `imagePullPolicy: Sometimes`). |

KL1004 compares the objects of one file, never objects in different files.

**Why:** repositories keep variants of the same object in separate files and
directories (a Kustomize base next to plain manifests, one directory per environment,
alternative examples), and only one of them is applied. On four public repositories
(956 objects), all 209 findings across files were such variants, and none was a bug.

**Rejected:** comparing every linted file (reports every overlay and environment);
comparing the files of one directory (65 of those findings were variants side by side
in one directory, such as `pod-nginx.yaml` and `pod-nginx-specific-node.yaml`).

KL1005 reports a requirement only when the template fails it as `kubectl create` stores
a null label (an empty value) and as `kubectl apply` stores it (the key is dropped). A
null selector value in `matchLabels` is never reported.

**Why:** the two commands disagree about a null, and the rule stays silent where they
disagree. A null selector value matches under `apply` and is empty under `create`; a
null template label is an empty value under `create` and absent under `apply`. Both
agree that a quoted `"null"` is text and that `null` in `values` is an empty value.

**Rejected:** comparing the text of a null (reports `null` against `""`, which both
commands accept, and misses `"null"` against `null`, which both reject); treating a null
as an empty string only (reports a null selector against a non-empty label, which
`apply` accepts); reporting when either command rejects (reports manifests that one of
them accepts).

Known costs: the cases where the commands split are silent: `values: [null]` or `[""]`
in an `In` expression against a null template label, a selector `k: ""` against `k:`,
`NotIn [null]` or `DoesNotExist` against `k: null`, a selector `k: null` against no
label, `"null"` or `x`. On 1242 generated cases, 140 that `apply` rejects and `create`
accepts and 106 that `create` rejects and `apply` accepts were reported before and are
silent now.

KL1004 and KL1008 know the scope of the kinds kubelint lists only. On a listed
cluster-scoped kind (Namespace, ClusterRole, StorageClass and so on) the written
`metadata.namespace` is not part of the identity and is not checked. On any other kind,
custom resources and built-in kinds kubelint does not list (Node, RuntimeClass), both
stay as written.

**Why:** the API server ignores the namespace of a cluster-scoped object, so two
ClusterRoles that differ only in `namespace` are one object. A custom resource's scope
is in its CustomResourceDefinition, which kubelint does not read.

**Rejected:** treating unknown kinds as cluster-scoped (it misses real duplicates and bad
namespaces on custom resources, which are mostly namespaced); reading the scope from a
CRD in the same file (a CRD is usually applied from another file).

KL1008 checks a `metadata.generateName` with the name's format, as a prefix that may end
with `-`. When the object has no `name`, it also checks the name the server generates
(the first 58 characters of the prefix and 5 more) and gives a CronJob prefix at most 47
characters. It stays silent when the answer depends on the Kubernetes version: a Service
prefix that starts with a digit, and a ReplicationController prefix when `name` is set.

**Why:** without a `name`, every create validates the generated name. With a `name`, the
server validates only the prefix characters and length of the prefix, and 1.34 alone
also rejects a bad ReplicationController prefix. There is no `--kubernetes-version`
flag yet, and a false positive is worse than a miss.

**Rejected:** checking a Service prefix as DNS-1035 (a false positive on 1.36 and later);
checking the generated name and the CronJob limit when `name` is set (the server accepts
`a.-` and a 60 character CronJob prefix then).

KL1008 and KL1010 skip a name, `generateName`, namespace, label key or label value with a
template placeholder: `{{`, or `$` followed by `{`, `(`, a letter or `_`.

**Why:** manifests kept as templates for tools other than Helm (envsubst, Jinja,
shell scripts) are linted as they are. A placeholder makes the value invalid only
until it is rendered, and the conventions say a rule that cannot decide stays silent.

**Rejected:** reporting placeholders (every templated name becomes an error); skipping
the whole object (hides findings that do not depend on the placeholder).

Known costs: the literal text around a placeholder is not checked either
(`My_App-{{x}}` is silent although no rendering fixes it), and a `$NAME` in a manifest
that no tool renders is silent too, although Kubernetes expands `$(VAR)` only in
`command`, `args` and `env`, never in metadata.

A document with no `metadata` key is more likely a configuration file than an object, so
KL1001 reports its missing `apiVersion` only when its `kind` is a built-in kind, KL1002
does not report its missing `kind`, and KL1003 reports its missing name only when its
`kind` is a built-in kind. A field written with an empty value (`kind:`) and an item of a
`kind: List` are reported as before.

**Why:** on public repositories, KL1001 reported a Kustomize `kustomization.yaml` (it has
`kind` and no `apiVersion`, and `kubectl kustomize` accepts it) and KL1002 reported a Helm
repository `index.yaml` (it has `apiVersion` and no `kind`). Neither is a Kubernetes
object, and `kubelint check .` is meant to skip YAML that is not Kubernetes. A manifest
that forgot `kind` nearly always has `metadata`, so the missing `metadata` is the signal
that separates the two.

**Rejected:** a list of known non-Kubernetes kinds (`Kustomization`, `Component`,
`Config`, ...), which never ends and misses every tool kubelint has not heard of;
checking file names (`kustomization.yaml`, `index.yaml`), which misses the same files
under other names, such as a chart description kept in another directory.

Known costs: a document that has neither `metadata` nor one of the two type fields is
silent unless its `kind` is a built-in kind written exactly (`kind: deployment` with no
`apiVersion`, or `apiVersion: apps/v1` with only a `spec`), although kubectl refuses it.
A `kustomization.yaml` that has `metadata` and no `apiVersion` is still reported.

## Security

| Code | Name | Default | Severity | Status | Milestone | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL2001 | `privileged-container` | on | error | implemented | M2 | `securityContext.privileged: true`. |
| KL2002 | `privilege-escalation-allowed` | off | warning | implemented | M2 | `allowPrivilegeEscalation` is not set to `false`. |
| KL2003 | `run-as-root` | off | warning | implemented | M2 | The `restricted` checks `runAsNonRoot` and `runAsUser`: `runAsNonRoot` is not `true` or is `false`, or `runAsUser` is `0`, honoring pod-level defaults. |
| KL2004 | `host-namespace` | on | warning | implemented | M2 | `hostNetwork`, `hostPID` or `hostIPC` is enabled. |
| KL2005 | `host-path-volume` | on | warning | implemented | M2 | A `hostPath` volume exposes the node's filesystem. |
| KL2006 | `dangerous-capability` | on | warning | implemented | M2 | Adds a Linux capability outside the `baseline` Pod Security Standard list, like `SYS_ADMIN`, `NET_ADMIN` or `ALL`. |
| KL2007 | `docker-socket-mount` | on | error | implemented | M2 | A `hostPath` volume mounts the container runtime socket (`docker.sock`, `containerd.sock`, `crio.sock`...). |
| KL2008 | `secret-in-env-literal` | on | warning | implemented | M2 | An env var whose name ends in a secret word (`PASSWORD`, `TOKEN`, `API_KEY`...) has a literal value instead of `valueFrom.secretKeyRef`. |
| KL2009 | `writable-root-filesystem` | off | warning | implemented | M2 | `readOnlyRootFilesystem` is not `true`. |

KL2002 `privilege-escalation-allowed` and KL2003 `run-as-root` ship off by default,
like KL2009 `writable-root-filesystem`. They check requirements of the `restricted` Pod
Security Standard, while the security rules that ship on check what the `baseline`
standard forbids. Select them by name or code (`--select KL2002` or `select` in
`kubelint.toml`) to turn them on.

**Why:** `baseline` findings are rare and serious (a privileged container, a host
mount), so each one is worth acting on. KL2002 and KL2003 fire on almost every container
that does not harden itself: on four public manifest collections (956 objects) KL2002
matched 415 times and KL2003 matched 426. A default that warns on nearly every object
trains users to ignore the tool, which the vision's low-noise pillar forbids. A team
that wants the `restricted` standard opts in, the same choice it makes on the cluster
with Pod Security Admission.

**Rejected:** shipping them on, which floods a first run; shipping only one of them on,
which splits one standard with no principled line between the two.

KL2003 `run-as-root` is the `restricted` Pod Security checks about root, `runAsNonRoot`
and `runAsUser`. It agrees with Pod Security admission on every manifest shape its
fixture and the audit cases cover, on Kubernetes 1.34 to 1.37 except `hostUsers: false`
(see below). It reports a container that does not set `runAsNonRoot: true` (a non-zero
`runAsUser` alone is reported), an explicit `runAsNonRoot: false` on the pod or a
container, and `runAsUser: 0` on the pod or a container (also spelled `0.0` or `0e0`,
which the API server reads as 0), including a pod-level value that every container
overrides. Windows pods are checked like the others, and a pod with `hostUsers: false`
is skipped, as `restricted` accepts it from 1.35. Until `--kubernetes-version` exists
the rule follows the newest version.

**Why:** the rule is off by default and documented as a `restricted` requirement, so a
user who turns it on expects it to agree with the cluster. The rule and the end to end
CI item then use one definition, and the cases the old rule passed (an explicit `false`
next to `runAsUser: 1000`, a pod `false` that a container overrides, a pod `runAsUser: 0`
that every container overrides, a Windows pod without `runAsNonRoot`) are rejected by
the server and now reported.

**Rejected:**

- Keeping "the container may run as root" and changing the catalog row and the end to
  end CI item to match. A non-zero `runAsUser` without `runAsNonRoot` runs as non-root,
  so this is the more precise question, but it answers a different one than the
  standard the rule's off-by-default choice rests on, and it would keep passing an
  explicit `runAsNonRoot: false` next to a non-zero `runAsUser`, a pod `false` that a
  container overrides, a pod `runAsUser: 0` that every container overrides and a Windows
  pod without `runAsNonRoot`, which the cluster rejects.
- Two rules, one per Pod Security check. It would split one standard over two codes and
  two selections, and both checks answer the same question for the user.

KL2002 `privilege-escalation-allowed` keeps skipping a privileged container and a
container that adds `CAP_SYS_ADMIN`, although `restricted` reports both as
`allowPrivilegeEscalation != false`.

**Why:** Kubernetes rejects `allowPrivilegeEscalation: false` next to either setting, so
KL2002's help could not be followed. KL2001 and KL2006 already report both containers,
with the fix that works (remove the setting).

**Rejected:** reporting them in KL2002 as well, which gives one container two findings
and one of them with an impossible fix.

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

KL2008 also skips a value that is a switch (`0`, `1`, `true`, `false`, `yes`, `no`,
`on`, `off`, in any case).

**Why:** names like `MYSQL_ALLOW_EMPTY_PASSWORD` end in a secret word but hold a
switch. A password that is one of these words (`DB_PASSWORD: "true"`) protects
nothing, so skipping it costs less than reporting every switch.

**Rejected:** a list of switch name prefixes (`ALLOW_`, `ENABLE_`, `REQUIRE_`), which
misses `MYSQL_RANDOM_ROOT_PASSWORD` and every prefix it does not list.

## Reliability

| Code | Name | Default | Severity | Status | Milestone | What it catches |
| --- | --- | --- | --- | --- | --- | --- |
| KL3001 | `missing-memory-limit` | on | warning | implemented | M2 | A container has no `resources.limits.memory`. |
| KL3002 | `missing-resource-requests` | on | warning | implemented | M2 | A container has no CPU or memory `requests`. |
| KL3003 | `unpinned-image-tag` | on | warning | implemented | M2 | An image uses `:latest` or no tag at all (digests are fine). |
| KL3004 | `missing-readiness-probe` | off | info | implemented | M2 | A long-running workload container has no `readinessProbe`. |
| KL3005 | `single-replica` | off | info | implemented | M2 | A Deployment or StatefulSet sets `replicas: 1`. |

KL3004 checks the regular containers of Deployments, StatefulSets, DaemonSets,
ReplicaSets and ReplicationControllers only.

**Why:** these run long and are what Services route to. Jobs and CronJobs run to
completion, so readiness gates nothing. Nothing rolls out a bare Pod, and bare Pods are
rare in production. Sidecars are usually injected or third-party, so their authors, not
the manifest's, choose their probes. Kubernetes rejects `readinessProbe` on plain init
and ephemeral containers.

**Rejected:** checking bare Pods (noise from test and debug pods); checking sidecars
(findings the manifest author often cannot fix); checking Jobs and CronJobs.

KL3005 reports only an explicit `replicas: 1`, not a missing `replicas`, although
Kubernetes defaults a missing one to 1.

**Why:** a HorizontalPodAutoscaler owns `replicas` when one targets the workload, and
its documentation asks manifests to leave the field out. The autoscaler often lives in
another file, chart or repository, so reporting a missing field would flag every
autoscaled workload.

**Rejected:** reporting a missing `replicas` (noise on every autoscaled workload);
checking ReplicaSets and ReplicationControllers (a ReplicaSet is usually owned by a
Deployment, and ReplicationControllers are legacy).
