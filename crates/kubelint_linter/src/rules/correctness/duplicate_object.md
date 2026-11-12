## What it does

Reports an object whose API group, `kind`, `metadata.namespace` and `metadata.name` are
the same as those of an object defined earlier in the same file.

## Why it matters

Kubernetes identifies an object by its kind, namespace and name. When two manifests
define the same object, `kubectl apply` applies both, one after the other, and the last
one silently wins. Which one that is depends on file order, so a change to the first
definition may never reach the cluster. This usually comes from copying a manifest and
forgetting to rename it.

Objects without a namespace are only compared with other objects without a namespace,
because their namespace is chosen when they are applied.

Copies in different files are not reported. Repositories often keep several variants of
the same object in separate files and directories (a Kustomize base and plain
manifests, one directory per environment, alternative examples), and only one of them
is applied.

## Example

```yaml
apiVersion: v1
kind: Service
metadata:
  name: web
---
apiVersion: v1
kind: Service
metadata:
  name: web
```

Use instead:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: web
---
apiVersion: v1
kind: Service
metadata:
  name: web-internal
```
