## What it does

Reports a YAML syntax error, such as a bad indentation, an unclosed bracket or quote, or
a tab used for indentation. This rule cannot be disabled.

## Why it matters

Kubernetes cannot read a document with a syntax error, and `kubectl apply` rejects the
whole file. kubelint cannot check the broken document either: it skips it and continues
with the next document after a `---` line.

## Example

```yaml
apiVersion: v1
kind: ConfigMap
data:
  greeting: hello
 farewell: bye
```

`farewell` is indented less than `greeting` but more than `data`, so it belongs to
neither. Use instead:

```yaml
apiVersion: v1
kind: ConfigMap
data:
  greeting: hello
  farewell: bye
```
