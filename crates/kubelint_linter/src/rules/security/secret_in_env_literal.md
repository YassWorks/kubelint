## What it does

Reports an env var whose name ends in a secret word and whose `value` is a literal
string, in `containers`, `initContainers` and `ephemeralContainers`. The secret words
are `PASSWORD`, `PASSWD`, `SECRET`, `TOKEN`, `API_KEY`, `SECRET_KEY`, `PRIVATE_KEY`,
`ACCESS_KEY` and `CREDENTIALS`, in any case and with or without `_`, `-` or `.`
between the words (`apiKey` and `api-key` count).

Only the end of the name counts: `TOKEN_URL`, `PASSWORD_FILE` and `AWS_ACCESS_KEY_ID`
are not reported. Neither are empty or whitespace-only values, values that are a
single `$(VAR)` or `${VAR}` reference, and switches that turn a secret feature on or
off (`MYSQL_ALLOW_EMPTY_PASSWORD: "1"`): `0`, `1`, `true`, `false`, `yes`, `no`, `on`
and `off`, in any case.

## Why it matters

A value written in the manifest is stored in plain text wherever the manifest goes: the
Git history, CI logs, `kubectl get -o yaml` output and every backup of the cluster's
objects. Anyone who can read the Deployment can read the password. Kubernetes Secrets
can be limited with RBAC, encrypted at rest and rotated without changing the workload.

## Example

```yaml
containers:
- name: app
  image: example/app:1.4
  env:
  - name: DB_PASSWORD
    value: hunter2
```

Use instead:

```yaml
containers:
- name: app
  image: example/app:1.4
  env:
  - name: DB_PASSWORD
    valueFrom:
      secretKeyRef:
        name: app-db
        key: password
```
