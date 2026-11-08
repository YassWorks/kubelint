## What it does

Reports an env var whose `name` is already set earlier in the same container's `env`
list.

## Why it matters

Kubernetes accepts the object with only a warning, and the container sees the last
value. The earlier entry is dead code that looks authoritative: someone edits it, and
nothing changes. Duplicates also break `kubectl apply --server-side`, which treats `env`
as a list keyed by `name`. This usually happens when a variable is added to a long
`env` list without noticing it is already there.

Variables that come from `envFrom` are not checked.

## Example

```yaml
containers:
- name: web
  image: nginx:1.27
  env:
  - name: LOG_LEVEL
    value: info
  - name: LOG_LEVEL
    value: debug
```

Use instead:

```yaml
containers:
- name: web
  image: nginx:1.27
  env:
  - name: LOG_LEVEL
    value: debug
```
