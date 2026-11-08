## What it does

Reports a Kubernetes object that has no `apiVersion` field, or whose `apiVersion` is
empty.

## Why it matters

`apiVersion` and `kind` together tell Kubernetes what an object is. Without
`apiVersion`, `kubectl apply` fails with "apiVersion not set" and nothing in the file is
applied.

## Example

```yaml
kind: Deployment
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

`kubectl api-resources` lists the `apiVersion` of every kind your cluster serves.
