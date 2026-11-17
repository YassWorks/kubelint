## What it does

Reports a container whose `securityContext.privileged` is `true`, in `containers`,
`initContainers` and `ephemeralContainers`.

## Why it matters

A privileged container has every Linux capability, sees every device on the node and is
not confined by seccomp or AppArmor. Anyone who controls the process controls the node:
they can mount the host filesystem, load kernel modules and read the secrets of every
other pod scheduled there. The `baseline` Pod Security Standard forbids it.

Very few workloads need it. Most that ask for it need one capability or one device. A
server that binds a port below 1024 as a non-root user needs only `NET_BIND_SERVICE`,
which `baseline` allows. A capability outside the `baseline` list, such as `NET_ADMIN`
for network setup or `SYS_PTRACE` for a debugger, is rejected in a namespace that
enforces `baseline`, and KL2006 reports it: a workload that needs one must run in a
namespace at the `privileged` Pod Security level. If the container really must be
privileged (some CNI or storage drivers), silence the finding for that object and say
why.

## Example

```yaml
containers:
- name: web
  image: example/web:1.4
  securityContext:
    privileged: true
```

Use instead:

```yaml
containers:
- name: web
  image: example/web:1.4
  securityContext:
    capabilities:
      drop: ["ALL"]
      add: ["NET_BIND_SERVICE"]
```
