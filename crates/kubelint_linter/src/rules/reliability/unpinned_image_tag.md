## What it does

Reports a container image that has no digest and either no tag or the `latest` tag,
such as `nginx` or `nginx:latest`. An image without a tag means `:latest`. A registry
port is not a tag: `localhost:5000/app` has no tag. An image pinned by digest
(`nginx@sha256:...`) is never reported, with or without a tag. Values that are not
image references, such as templates (`{{ .Values.image }}`), are skipped.

## Why it matters

`latest` is not a version: it points at whatever was pushed last. Two nodes can run
different code for the same pod, a restart can silently upgrade the application, and a
rollback to the previous manifest does not bring the previous code back. For a
`latest` or untagged image Kubernetes also defaults `imagePullPolicy` to `Always`, so
every pod start depends on the registry being reachable. A version tag makes each
deployment reproducible, and a digest also guarantees that nobody can move the tag.

## Example

```yaml
containers:
- name: web
  image: nginx:latest
```

Use instead:

```yaml
containers:
- name: web
  image: nginx:1.27
```
