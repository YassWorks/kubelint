#!/usr/bin/env bash
set -euo pipefail

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit --message "chore(release): v$VERSION"
git tag "v$VERSION"
git push --atomic origin HEAD:main "v$VERSION"
