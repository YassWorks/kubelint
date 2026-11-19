#!/usr/bin/env bash
#
# Runs `cargo xtask e2e` against an API server of $KUBERNETES_VERSION.
set -euo pipefail

cargo build --package xtask --locked
eval "$(bash .github/scripts/e2e-apiserver.sh "$KUBERNETES_VERSION")"
cargo xtask e2e
