#!/usr/bin/env bash
set -euo pipefail

read -rd '' -a args <<< "$ARGS" || true
if [ "$FORMAT" = sarif ]; then
  kubelint check --format sarif ${args[@]+"${args[@]}"} > "$SARIF_FILE"
else
  kubelint check --format "$FORMAT" ${args[@]+"${args[@]}"}
fi
