## What it does

Reports a Kubernetes object that has no `kind` field, or whose `kind` is empty.

## Why it matters

`apiVersion` and `kind` together tell Kubernetes what an object is. Without `kind`,
`kubectl apply` fails with "kind not set". It applies the documents before that object,
stops there, and applies none after it. An empty `kind` behaves differently: kubectl
reports that the object has no kind, and the documents after it are still applied.

Files that only look like objects, such as a Helm repository index or a `Chart.yaml`,
are not reported when they have no `metadata` key and no `kind` key. An empty `kind:`
and an item of a `kind: List` are always reported.

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
