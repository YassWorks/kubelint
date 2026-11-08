#!/usr/bin/env bash
# Builds kubelint for TARGET at VERSION and packages it into dist/.
# main keeps version 0.0.0; the release version exists only in this build.
set -euo pipefail

perl -pi -e "s/^version = \"0\.0\.0\"/version = \"$VERSION\"/" Cargo.toml
grep -q "^version = \"$VERSION\"" Cargo.toml

cargo build --release --package kubelint --target "$TARGET"

name="kubelint-v$VERSION-$TARGET"
mkdir -p "dist/$name"
cp LICENSE README.md "dist/$name/"
if [[ "$TARGET" == *windows* ]]; then
  cp "target/$TARGET/release/kubelint.exe" "dist/$name/"
  (cd dist && 7z a -bd "$name.zip" "$name" > /dev/null)
else
  cp "target/$TARGET/release/kubelint" "dist/$name/"
  tar -czf "dist/$name.tar.gz" -C dist "$name"
fi
