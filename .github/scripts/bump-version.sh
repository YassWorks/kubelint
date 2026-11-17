#!/usr/bin/env bash
set -euo pipefail

sed -i -E "/^\[workspace\.package\]/,/^\[/ s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
grep -q "^version = \"$VERSION\"$" Cargo.toml
cargo update --workspace
