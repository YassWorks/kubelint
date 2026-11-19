#!/usr/bin/env bash
set -euo pipefail

exports="$(bash .github/scripts/e2e-apiserver.sh "$KUBERNETES_VERSION")"
eval "$exports"
cargo run --locked --package xtask -- e2e
