#!/usr/bin/env bash
# Runs kubelint for the GitHub Action (action.yml). Reads FORMAT, ARGS and SARIF_FILE.
# ARGS is split on spaces and newlines, like a command line without quotes.
set -euo pipefail

read -rd '' -a args <<< "$ARGS" || true
if [ "$FORMAT" = sarif ]; then
  kubelint check --format sarif ${args[@]+"${args[@]}"} > "$SARIF_FILE"
else
  kubelint check --format "$FORMAT" ${args[@]+"${args[@]}"}
fi
