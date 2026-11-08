#!/usr/bin/env bash
# Publishes the archives in dist/ as GitHub Release vVERSION, with notes grouped by
# Conventional Commit type. Reads VERSION, PREVIOUS, CODE_PATHS and GITHUB_SHA.
set -euo pipefail

range="${PREVIOUS:+$PREVIOUS..}HEAD"

section() {
  local lines
  lines="$(git log "$range" --format=%s -- $CODE_PATHS | grep -E "$2" \
    | sed -E 's/^[a-z]+(\([^)]*\))?!?: */- /' || true)"
  if [ -n "$lines" ]; then printf '## %s\n\n%s\n\n' "$1" "$lines"; fi
}

{
  section "Features" '^feat(\(.+\))?!?:'
  section "Fixes" '^fix(\(.+\))?!?:'
  section "Performance" '^perf(\(.+\))?!?:'
  section "Other changes" '^(refactor|chore|build|ci|test|docs|style)(\(.+\))?!?:'
} > notes.md
cat notes.md

(cd dist && sha256sum kubelint-* > SHA256SUMS)

gh release create "v$VERSION" dist/* \
  --target "$GITHUB_SHA" \
  --title "v$VERSION" \
  --notes-file notes.md
