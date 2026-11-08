## What it does

Reports a `metadata.name` that is not valid for the object's kind, and a
`metadata.namespace` that is not a valid namespace name.

## Why it matters

Kubernetes uses names in DNS records, URLs and file paths, so it restricts them. Most
objects need a DNS-1123 subdomain: lowercase letters, digits, `-` and `.`, at most 253
characters, starting and ending with a letter or digit. Namespaces need a DNS-1123
label (no dots, at most 63 characters), and Services a DNS-1035 label, which must also
start with a letter. The API server rejects any other name, so names like `My_App` or
`web.` fail at apply time. Uppercase letters and underscores are the usual culprits.

## Example

```yaml
kind: Deployment
metadata:
  name: My_App
```

Use instead:

```yaml
kind: Deployment
metadata:
  name: my-app
```
