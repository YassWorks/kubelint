## What it does

Reports a Kubernetes object that has no `metadata.name`, or whose name is empty. An
object with `metadata.generateName` instead is fine.

## Why it matters

Every object stored by Kubernetes needs a name: it is how `kubectl`, controllers and
other objects refer to it. `kubectl apply` fails with "resource name may not be empty"
and nothing in the file is applied.

`generateName` asks the API server to make up a unique name from a prefix. It works with
`kubectl create`, but not with `kubectl apply`, which needs to know the name to find the
object again.

Files that only look like objects, such as `kustomization.yaml` or a kubeconfig, are
not reported when they have no `metadata` at all.

## Example

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  labels:
    app: web
```

Use instead:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
  labels:
    app: web
```
