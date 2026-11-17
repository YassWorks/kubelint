## What it does

Reports a key that appears more than once in the same YAML mapping, anywhere in a
Kubernetes object. Keys are compared by their text after removing quotes, so `"app"`
and `app` are the same key. A key that a merge key (`<<: *anchor`) writes again is not a
duplicate: the later of the two wins, as Kubernetes reads it. Keys inside a value that a
later key or a merge replaces are not checked, because Kubernetes never reads that value.

## Why it matters

Kubernetes cannot keep both values. `kubectl` uses the last one and says nothing, in
every mode it has, so the earlier value is dropped without a warning. Only a raw API
request with strict field validation rejects the duplicate. A reader of the file cannot
tell which value was meant.

## Example

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: settings
data:
  mode: fast
data:
  mode: slow
```

Use instead, keeping the value you mean:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: settings
data:
  mode: slow
```
