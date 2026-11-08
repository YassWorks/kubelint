## What it does

Reports a container whose `name` is already used by another container in the same pod,
counting `containers`, `initContainers` and `ephemeralContainers` together.

## Why it matters

Kubernetes refers to containers by name: in `kubectl logs`, `kubectl exec`, status
reports and probes. Names must be unique within a pod, so the API server rejects the
object with "Duplicate value". This usually happens when a container block is copied to
add a sidecar and the name is not changed.

## Example

```yaml
spec:
  containers:
  - name: web
    image: nginx:1.27
  - name: web
    image: envoyproxy/envoy:v1.31.0
```

Use instead:

```yaml
spec:
  containers:
  - name: web
    image: nginx:1.27
  - name: proxy
    image: envoyproxy/envoy:v1.31.0
```
