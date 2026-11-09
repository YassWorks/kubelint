## What it does

Reports each capability in a container's `securityContext.capabilities.add` that the
`baseline` Pod Security Standard does not allow, in `containers`, `initContainers` and
`ephemeralContainers`. `ALL` is reported too.

The `baseline` profile allows adding only `AUDIT_WRITE`, `CHOWN`, `DAC_OVERRIDE`,
`FOWNER`, `FSETID`, `KILL`, `MKNOD`, `NET_BIND_SERVICE`, `SETFCAP`, `SETGID`,
`SETPCAP`, `SETUID` and `SYS_CHROOT`, written exactly so. Pod Security Admission
rejects `net_bind_service` or `CAP_NET_BIND_SERVICE`, so KL2006 reports them too, with
the spelling to use.

## Why it matters

Linux capabilities split the power of root into pieces. The pieces outside the
`baseline` list each give a way out of the container or into its neighbors:
`SYS_ADMIN` can mount filesystems and is close to `privileged: true`, `NET_ADMIN`
can reconfigure the pod's network and sniff traffic, `SYS_PTRACE` can read the memory
of other processes, `SYS_MODULE` can load kernel modules. `ALL` adds every one of them.

If the container really needs one (a VPN client needs `NET_ADMIN`), keep only that one,
drop the others with `drop: ["ALL"]`, and silence the finding for that object with the
reason.

## Example

```yaml
containers:
- name: app
  image: example/app:1.4
  securityContext:
    capabilities:
      add: ["SYS_ADMIN", "NET_BIND_SERVICE"]
```

Use instead:

```yaml
containers:
- name: app
  image: example/app:1.4
  securityContext:
    capabilities:
      drop: ["ALL"]
      add: ["NET_BIND_SERVICE"]
```
