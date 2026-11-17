## What it does

Reports a `metadata.name` or `metadata.generateName` that is not valid for the object's
kind, and a `metadata.namespace` that is not a valid namespace name.

Names are checked for the common built-in kinds kubelint knows, such as Deployment,
Service, ConfigMap, Namespace and the RBAC kinds. The names of other kinds, including
custom resources, are not checked yet. Their namespace is.

The namespace is not checked on a cluster-scoped kind (Namespace, ClusterRole,
StorageClass and so on), because the API server ignores it there. kubelint knows the
scope of the built-in kinds it lists only. For any other kind, such as a custom resource,
whose scope is in its definition, the namespace is checked.

A `generateName` is the start of a name: the server adds 5 random characters, so it may
end with `-` and is checked with the same format as the name. The `-` does not count
towards the length. When the object has no `name`, the server also checks the name it
generates (the first 58 characters of the prefix and 5 more), so `a.-` fails there.

A name, `generateName` or namespace with a template placeholder (`{{ .Values.name }}`,
`${NAME}`, `$(NAME)`, `$NAME`) is not reported, because its final text is only known
once the template is rendered.

## Why it matters

Kubernetes uses names in DNS records, URLs and file paths, so it restricts them. The API
server rejects a name that breaks the rules below, so names like `My_App` or `web.` fail
at apply time. Uppercase letters and underscores are the usual culprits.

| Kind | Allowed names |
| --- | --- |
| Most kinds | Lowercase letters, digits, `-` and `.`, at most 253 characters. Each part between dots starts and ends with a letter or digit. |
| Namespace, StatefulSet | Lowercase letters, digits and `-`, at most 63 characters, starting and ending with a letter or digit. No dots. |
| Service | The same as a Namespace, and it must start with a letter. |
| CronJob | The same as most kinds, and at most 52 characters. Without a `name`, a `generateName` has at most 47. |
| Job | The same as most kinds, and at most 63 characters unless `spec.manualSelector` is `true`. |
| Role, ClusterRole, RoleBinding, ClusterRoleBinding, PodDisruptionBudget | Any name except `.` and `..`, without `/` or `%`, so `system:controller:foo` is fine. |

Two `generateName` cases are not reported, because the API server's answer depends on its
version. A Service prefix that starts with a digit is accepted from Kubernetes 1.36 and
rejected before. A ReplicationController prefix with invalid characters is rejected only
by 1.34 when the object also has a `name`.

## Example

```yaml
kind: Deployment
metadata:
  name: My_App
```

Use instead:

```yaml
kind: Deployment
metadata:
  name: my-app
```
