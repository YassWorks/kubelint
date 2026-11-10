## What it does

Reports a container in `containers` of a Deployment, StatefulSet, DaemonSet, ReplicaSet
or ReplicationController that has no `readinessProbe`. Jobs and CronJobs are skipped
because they run to completion, and bare Pods because nothing rolls them out. Init and
ephemeral containers cannot have a readiness probe. Sidecars (init containers with
`restartPolicy: Always`) are skipped too, because they are usually injected or come from
a third party.

This rule is off by default, at `info` severity: a worker that serves no traffic has
nothing to signal, and some applications start serving the moment their process starts.
Enable it with `--select KL3004` or in `kubelint.toml`.

## Why it matters

Without a readiness probe, Kubernetes considers a container ready as soon as it is
running (or as soon as its `startupProbe` succeeds), and never unready again while it
runs. A Service then sends it requests while the application is still loading, warming
its caches or connecting to its database, and those requests fail. A Deployment,
StatefulSet or DaemonSet update also moves on once the new pod is running (plus
`minReadySeconds`), so a version that runs but cannot serve can replace every healthy
pod. A readiness probe holds traffic and the rollout back until the container says it
is ready, and takes it out of the Service again when it stops being able to serve.

## Example

```yaml
containers:
- name: web
  image: nginx:1.27
  ports:
  - containerPort: 80
```

Use instead:

```yaml
containers:
- name: web
  image: nginx:1.27
  ports:
  - containerPort: 80
  readinessProbe:
    httpGet:
      path: /
      port: 80
```
