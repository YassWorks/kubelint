## What it does

Reports a container in `containers` or `initContainers` that has no CPU request, no
memory request, or neither. A limit counts as a request, because Kubernetes copies a
limit into a missing request of the same resource. Pod-level `spec.resources` requests
or limits (Kubernetes 1.34 and later, `PodLevelResources`) cover that resource for every
container of the pod. Ephemeral containers are skipped, because Kubernetes does not
allow `resources` on them.

## Why it matters

The scheduler places pods by their requests. A container without requests counts as
needing nothing, so the scheduler can pack too many pods on one node, and they then
compete for CPU and memory under load. A container without a memory request is also
among the first to be evicted when the node runs low on memory, and a pod where no
container sets any request or limit gets the `BestEffort` quality of service class,
which is evicted before all others. Some namespaces set default requests with a
`LimitRange`, but the manifest cannot rely on it being there.

## Example

```yaml
containers:
- name: web
  image: nginx:1.27
```

Use instead:

```yaml
containers:
- name: web
  image: nginx:1.27
  resources:
    requests:
      cpu: 100m
      memory: 256Mi
```
