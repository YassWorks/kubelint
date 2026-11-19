#!/usr/bin/env bash
set -euo pipefail

eval "$(bash .github/scripts/e2e-apiserver.sh "$KUBERNETES_VERSION")"
cargo run --locked --package xtask -- e2e
