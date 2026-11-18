#!/usr/bin/env bash
set -euo pipefail

tag="$(jq --raw-output .announcement_tag <<<"$PLAN")"
context="$(mktemp -d)"
for platform in amd64:x86_64 arm64:aarch64; do
  IFS=: read -r arch cpu <<<"$platform"
  target="$cpu-unknown-linux-musl"
  mkdir "$context/$arch"
  curl -fsSL "$RELEASES/$tag/kubelint-$target.tar.gz" \
    | tar -xz -C "$context/$arch" --strip-components=1 "kubelint-$target/kubelint"
done
docker buildx build --platform linux/amd64,linux/arm64 --file .github/docker/Dockerfile \
  --tag "$IMAGE:${tag#v}" --tag "$IMAGE:latest" --push "$context"
