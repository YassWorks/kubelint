# Suppressions

A suppression tells kubelint that you have looked at a finding and accept it. There
are two kinds:

| Kind | Silences | Use it when |
| --- | --- | --- |
| [Comment](#comments) | a line, or a whole file | you edit the YAML by hand |
| [Annotation](#annotations) | one object | the YAML goes through a tool that drops comments, such as `kustomize build` |

To turn a rule off for the whole project, or for a set of files, use `ignore` or
`[files] exclude` in the [configuration file](configuration.md) instead.

Say why in each suppression. A comment takes a reason after `--`. For an annotation,
put the reason in a comment next to it, or in the commit that adds it. The next reader
will know the finding was a decision, not an oversight.

## Comments

```yaml
securityContext:
  # kubelint-disable-next-line KL2001 -- the CNI plugin needs this
  privileged: true
  allowPrivilegeEscalation: true  # kubelint-disable-line privilege-escalation-allowed
```

| Comment | Silences |
| --- | --- |
| `# kubelint-disable-next-line <rules>` | the next line that is not only a comment |
| `# kubelint-disable-line <rules>` | its own line |
| `# kubelint-disable-file <rules>` | the whole file, wherever the comment is |

- `<rules>` is a comma-separated list of rule codes or names. An optional `-- reason`
  follows.
- A finding is silenced when its highlighted range starts on a silenced line.
- `KL0001` `invalid-yaml` cannot be silenced. A comment that names an unknown rule, names
  `KL0001` or names no rule silences nothing, and `KL0003` `invalid-suppression` reports it.

**Rejected:** Python-style `# kubelint: ignore[KL2001]`. ESLint's style is more familiar
to most people who write manifests.

## Annotations

```yaml
metadata:
  annotations:
    kubelint/ignore: "KL2001,KL3003"
```

The `kubelint/ignore` annotation silences the listed rules for that one object.

- The value is a comma-separated list of rule codes or names, like a suppression
  comment, without a `-- reason`.
- A finding is silenced when its highlighted range starts inside the object. That
  covers findings about the object made while looking at other objects, such as
  `KL1004` `duplicate-object`, which points at the later of two duplicates.
- `KL0001` `invalid-yaml` cannot be silenced. An annotation that names an unknown rule,
  names `KL0001` or names no rule silences nothing, and `KL0003` `invalid-suppression`
  reports it. A value that is not a string is ignored.
- In a `kind: List`, each item is an object of its own: put the annotation on the item.
  An annotation on the List itself silences nothing.

**Why a range:** it covers every finding about the object, including the ones a rule
makes while comparing objects, with one simple test that does not depend on how a rule
is written.

**Rejected:** silencing a rule by not running it on the object. Simpler, but findings
from rules that look at every object at once, such as `KL1004`, would still appear.

**Why an annotation:** it survives tools that re-serialize YAML and drop comments, such as
`kustomize build`. `kubelint/ignore` is a valid annotation key that claims no domain.

**Rejected:**

- `kubelint.dev/ignore`. It follows the domain prefix convention, but with a domain the
  project does not own.
- One key per rule, such as `ignore.kubelint/KL2001: "reason"` (kube-linter's style).
  More verbose for the common case of silencing two or three rules.
