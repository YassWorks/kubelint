## What it does

Reports a key that appears more than once in the same YAML mapping, anywhere in a
Kubernetes object. Keys are compared by their text after removing quotes, so `"app"`
and `app` are the same key.

## Why it matters

Kubernetes cannot keep both values. Depending on the tool and its settings, the object
is rejected (`kubectl apply` with strict field validation reports "duplicate field") or
the last value silently wins. Either way, one of the two values is not what runs, and a
reader of the file cannot tell which one was meant.

## Example

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: settings
data:
  mode: fast
data:
  mode: slow
```

Use instead, keeping the value you mean:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: settings
data:
  mode: slow
```
