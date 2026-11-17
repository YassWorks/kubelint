#!/usr/bin/env bash
set -euo pipefail

if ! grep -q "^version = \"${GITHUB_REF_NAME#v}\"$" Cargo.toml; then
  echo "::error::Cargo.toml does not have the version of tag $GITHUB_REF_NAME"
  exit 1
fi

cargo build --release --locked --package kubelint --target "$TARGET"

name="kubelint-$GITHUB_REF_NAME-$TARGET"
mkdir -p "dist/$name"
cp LICENSE README.md "dist/$name/"
if [[ "$TARGET" == *windows* ]]; then
  cp "target/$TARGET/release/kubelint.exe" "dist/$name/"
  (cd dist && 7z a -bd "$name.zip" "$name" > /dev/null)
else
  cp "target/$TARGET/release/kubelint" "dist/$name/"
  tar -czf "dist/$name.tar.gz" -C dist "$name"
fi
