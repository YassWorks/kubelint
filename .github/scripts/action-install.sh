#!/usr/bin/env bash
set -euo pipefail

repo="https://github.com/YassWorks/kubelint"

if [ -z "$VERSION" ]; then
  if [[ "$ACTION_REF" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    VERSION="$ACTION_REF"
  else
    VERSION=latest
  fi
fi
if [ "$VERSION" = latest ]; then
  VERSION="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "$repo/releases/latest")"
  VERSION="${VERSION##*/}"
fi

case "$RUNNER_OS/$RUNNER_ARCH" in
  Linux/X64) target=x86_64-unknown-linux-musl ext=tar.gz ;;
  Linux/ARM64) target=aarch64-unknown-linux-musl ext=tar.gz ;;
  macOS/X64) target=x86_64-apple-darwin ext=tar.gz ;;
  macOS/ARM64) target=aarch64-apple-darwin ext=tar.gz ;;
  Windows/X64) target=x86_64-pc-windows-msvc ext=zip ;;
  *)
    echo "::error::kubelint has no release binary for $RUNNER_OS $RUNNER_ARCH"
    exit 1
    ;;
esac

name="kubelint-$target"
sums=sha256.sum
dir="$RUNNER_TEMP/kubelint"
mkdir -p "$dir"
cd "$dir"
if ! curl -fsLO "$repo/releases/download/$VERSION/$sums"; then
  name="kubelint-$VERSION-$target"
  sums=SHA256SUMS
  curl -fsSLO "$repo/releases/download/$VERSION/$sums"
fi
curl -fsSLO "$repo/releases/download/$VERSION/$name.$ext"
grep -E " \*?$name\.$ext\$" "$sums" > expected
if command -v sha256sum > /dev/null; then sha256sum -c expected; else shasum -a 256 -c expected; fi

if [ "$ext" = zip ]; then
  unzip -q -o -j "$name.$ext" -d "$name"
else
  tar -xzf "$name.$ext"
fi
echo "$dir/$name" >> "$GITHUB_PATH"
echo "Installed kubelint $VERSION ($target)."
