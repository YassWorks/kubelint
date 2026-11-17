#!/usr/bin/env bash
set -euo pipefail

titles="$(git log "${PREVIOUS:+$PREVIOUS..}HEAD" --format=%s -- $CODE_PATHS)"
type='(\([^)]*\))?!?:'

group() {
  local lines
  lines="$(grep "${@:3}" -E "$2" <<< "$titles" | sed -E "s/^[a-z]+$type *//; s/^/- /" || true)"
  if [ -n "$lines" ]; then printf '### %s\n\n%s\n\n' "$1" "$lines"; fi
}

section="$(
  printf '## [%s] - %s\n\n' "$VERSION" "$(date -u +%F)"
  group "Features" "^feat$type"
  group "Fixes" "^fix$type"
  group "Performance" "^perf$type"
  group "Other changes" "^(feat|fix|perf)$type" --invert-match
)"
header="$(sed '/^## /,$d' CHANGELOG.md)"
older="$(sed -n '/^## /,$p' CHANGELOG.md)"

{
  printf '%s\n\n%s\n' "$header" "$section"
  if [ -n "$older" ]; then printf '\n%s\n' "$older"; fi
} > CHANGELOG.md
echo "$section"
