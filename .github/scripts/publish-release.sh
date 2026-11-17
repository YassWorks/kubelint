#!/usr/bin/env bash
set -euo pipefail

awk -v heading="## [${GITHUB_REF_NAME#v}] " '
  index($0, heading) == 1 { found = 1; next }
  found && /^## / { exit }
  found
' CHANGELOG.md > notes.md
if ! grep -q . notes.md; then
  echo "::error::CHANGELOG.md has no section for $GITHUB_REF_NAME"
  exit 1
fi
cat notes.md

(cd dist && sha256sum kubelint-* > SHA256SUMS)

gh release create "$GITHUB_REF_NAME" dist/* \
  --verify-tag \
  --title "$GITHUB_REF_NAME" \
  --notes-file notes.md
