#!/usr/bin/env bash
# Writes the next release version and the previous tag to $GITHUB_OUTPUT.
# Reads CODE_PATHS. Writes no version when no code changed since the last release.
set -euo pipefail

previous="$(git tag --list 'v*' --sort=-v:refname | head -n1)"
subjects="$(git log "${previous:+$previous..}HEAD" --format=%s -- $CODE_PATHS)"

if [ -z "$subjects" ]; then
  echo "No code changes since ${previous:-the first commit}."
  exit 0
elif [ -z "$previous" ]; then
  version="0.1.0"
else
  IFS=. read -r major minor patch <<< "${previous#v}"
  if grep -qE '^[a-z]+(\(.+\))?!:' <<< "$subjects"; then
    if [ "$major" = 0 ]; then bump=minor; else bump=major; fi
  elif grep -qE '^feat(\(.+\))?:' <<< "$subjects"; then
    bump=minor
  else
    bump=patch
  fi
  case "$bump" in
    major) version="$((major + 1)).0.0" ;;
    minor) version="$major.$((minor + 1)).0" ;;
    patch) version="$major.$minor.$((patch + 1))" ;;
  esac
fi

echo "version=$version" >> "$GITHUB_OUTPUT"
echo "previous=$previous" >> "$GITHUB_OUTPUT"
echo "Next version: ${previous:-none} -> v$version"
