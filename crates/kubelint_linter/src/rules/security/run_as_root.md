## What it does

Reports what the `restricted` Pod Security Standard reports for its two checks about
root, `runAsNonRoot` and `runAsUser`. In `containers`, `initContainers`,
`ephemeralContainers` and the pod's own `securityContext`:

- `runAsNonRoot: false` on the pod or a container.
- `runAsUser: 0` on the pod or a container, even next to `runAsNonRoot: true`.
- A container that does not set `runAsNonRoot` and whose pod does not set it to `true`.
  A non-zero `runAsUser` alone is not enough: the check wants `runAsNonRoot: true`.

A container's `securityContext` overrides the pod's field by field, and a null value
falls back to the pod. A pod-level `runAsNonRoot: false` or `runAsUser: 0` is reported
once on the pod, even when every container overrides it, as Pod Security does.

Windows pods (`spec.os.name: windows`) are checked like any other pod: Pod Security has
no exemption, and Kubernetes rejects `runAsUser` and `hostUsers` on them, so
`runAsNonRoot: true` is the way to pass. A pod with `hostUsers: false` is skipped,
because `restricted` accepts any user in a user namespace from Kubernetes 1.35. The rule
follows the newest version until `--kubernetes-version` exists, so on 1.34 and older a
pod with `hostUsers: false` still fails the cluster check.

This rule is off by default: most manifests do not set this field, so it would warn on
nearly every container. Enable it with `--select KL2003` or in `kubelint.toml`.

## Why it matters

Unless the pod uses a user namespace (`hostUsers: false`), root in a container is the
same user as root on the node, held back only by the container's isolation. A bug in
the application, or a kernel or runtime vulnerability, then gives an attacker root on
the node instead of an unprivileged account. Root can also write anywhere in the image
and in mounted volumes.

With `runAsNonRoot: true`, the kubelet refuses to start a container whose user is root,
so a root image fails loudly instead of running quietly. If the image has no non-root
`USER`, also set `runAsUser` to a non-zero ID.

## Example

```yaml
spec:
  containers:
  - name: web
    image: nginx:1.27
```

Use instead:

```yaml
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 101
  containers:
  - name: web
    image: nginxinc/nginx-unprivileged:1.27
```

The `nginx` image needs root: as user 101 it can write neither its temporary files
under `/var/cache/nginx` nor its PID file under `/run`. Its unprivileged variant runs as
user 101 and listens on port 8080 instead of 80.
