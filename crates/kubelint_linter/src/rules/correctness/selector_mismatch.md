## What it does

Reports a Deployment, ReplicaSet, DaemonSet, StatefulSet, Job or ReplicationController
whose `spec.selector` does not match the labels in `spec.template.metadata.labels`.

## Why it matters

A workload finds the pods it owns with its selector. The pods it creates get the labels
of its pod template. If the template's labels do not satisfy the selector, the workload
could never see its own pods, so the API server rejects it with
"`selector` does not match template `labels`".

The selector of a Deployment, ReplicaSet, DaemonSet or StatefulSet cannot be changed
after it is created, so it is worth getting right before the first apply.

A null label value (`k:`, `k: ~`, `k: null`) is not the text `null`. `kubectl apply` drops
a null label and `kubectl create` stores it as an empty value, so the rule reports a
requirement only when the template fails it both ways. A quoted `"null"` is text.

## Example

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
spec:
  selector:
    matchLabels:
      app: web
  template:
    metadata:
      labels:
        app: frontend
```

Use instead:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
spec:
  selector:
    matchLabels:
      app: web
  template:
    metadata:
      labels:
        app: web
```
