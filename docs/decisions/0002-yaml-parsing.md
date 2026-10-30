# 0002. YAML parsing with saphyr-parser and our own tree

- Status: Accepted
- Date: 2026-10-30

## Context

A linter must point at the exact key or value that is wrong, so it needs a position for
every node. It must also see the file as written (duplicate keys, quoting style, comments
for suppressions) rather than a cleaned-up value. `serde_yaml` is archived and discards
positions. Typed Kubernetes structs (`k8s-openapi`) discard positions and fail to
deserialize exactly the broken input a linter exists to explain.

## Decision

- Parse with **`saphyr-parser`** (the maintained successor of `yaml-rust`/`yaml-rust2`),
  using its event stream, which carries a span for every scalar and collection boundary.
- Build **our own tree** in `kubelint_yaml` with a `TextRange` (byte offsets) on every
  node, mapping entries kept in source order including duplicates. No `saphyr` type
  appears in our public API, so the parser can be replaced without touching rules.
- **Convert positions to bytes.** `saphyr-parser` 0.3.3 reports character indices
  (verified 2026-10-30). The builder converts them, and tests always include non-ASCII.
- **Kubernetes scalar semantics.** Typing helpers follow the YAML 1.1 style rules used by
  Kubernetes' decoder (`yes`/`no`/`on`/`off` are booleans), not the YAML 1.2 core schema.
- **Comments** are recovered by scanning the source outside scalar ranges, because the
  event stream does not include them.

## Alternatives considered

- **`saphyr` (the higher-level crate) `MarkedYaml`.** Gives spans, but less control over
  duplicate keys, key spans and styles, and ties our API to its types.
- **`yaml-rust2`.** Maintained, but the same maintainers now focus on `saphyr`.
- **Writing our own YAML parser.** Maximum control (lossless CST, exact comments), very
  high cost. Revisit only if autofix or the language server proves the event approach
  insufficient, via a new ADR.

## Consequences

- Rules work with precise ranges from day one; autofix can compute byte edits.
- We own a few hundred lines of tree-building code and its tests.
- Formatting-preserving autofix (M5) will need care: edits are applied to the original
  text by range, never by re-serializing the tree.
