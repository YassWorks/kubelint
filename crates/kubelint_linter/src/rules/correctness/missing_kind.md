## What it does

Reports a Kubernetes object that has no `kind` field, or whose `kind` is empty.

## Why it matters

`apiVersion` and `kind` together tell Kubernetes what an object is. Without `kind`,
`kubectl apply` fails with "kind not set" and nothing in the file is applied.

## Example

```yaml
apiVersion: apps/v1
metadata:
  name: web
```

Use instead:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
```
