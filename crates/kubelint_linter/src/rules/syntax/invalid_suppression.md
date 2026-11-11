## What it does

Checks `# kubelint-disable-next-line`, `# kubelint-disable-line` and
`# kubelint-disable-file` comments, and `kubelint/ignore` annotations. It reports a
rule code or name that no rule has, a suppression that names `KL0001` `invalid-yaml`
(syntax errors cannot be suppressed), and a suppression that names no rule at all.

## Why it matters

A suppression with a typo silences nothing, so the finding it was meant to hide is still
reported, or, worse, the suppression looks like it documents an accepted risk that nobody
actually reviewed. Reporting it keeps suppressions honest.

## Example

```yaml
securityContext:
  # kubelint-disable-next-line privileged-containers
  privileged: true
```

The rule is called `privileged-container`. Use instead:

```yaml
securityContext:
  # kubelint-disable-next-line privileged-container -- the CNI plugin needs this
  privileged: true
```
