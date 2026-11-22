#!/bin/sh
# Install the latest (or a pinned) whylog release.
#   curl -fsSL https://raw.githubusercontent.com/stuyk/whylog/main/install.sh | sh
# Env: WHYLOG_VERSION=0.1.0 (default: latest), WHYLOG_INSTALL_DIR (default: ~/.local/bin)
set -eu

repo="stuyk/whylog"
dir="${WHYLOG_INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
  Linux) os=linux ;;
  Darwin) os=darwin ;;
  *) echo "whylog: unsupported OS $(uname -s); see https://github.com/$repo/releases (Windows: install.ps1)" >&2; exit 1 ;;
esac
case "$(uname -m)" in
  x86_64|amd64) arch=amd64 ;;
  arm64|aarch64) arch=arm64 ;;
  *) echo "whylog: unsupported arch $(uname -m)" >&2; exit 1 ;;
esac

if [ -n "${WHYLOG_VERSION:-}" ]; then
  version="${WHYLOG_VERSION#v}"
else
  version="$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" | sed -n 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/p' | head -n1)"
  [ -n "$version" ] || { echo "whylog: could not determine latest release" >&2; exit 1; }
fi

archive="whylog_${version}_${os}_${arch}.tar.gz"
base="https://github.com/$repo/releases/download/v$version"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "downloading whylog v$version ($os/$arch)..."
curl -fsSL "$base/$archive" -o "$tmp/$archive"
curl -fsSL "$base/checksums.txt" -o "$tmp/checksums.txt"

expected="$(grep " $archive\$" "$tmp/checksums.txt" | cut -d' ' -f1)"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$tmp/$archive" | cut -d' ' -f1)"
else
  actual="$(shasum -a 256 "$tmp/$archive" | cut -d' ' -f1)"
fi
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
  echo "whylog: checksum mismatch for $archive" >&2; exit 1
fi

tar -xzf "$tmp/$archive" -C "$tmp" whylog
mkdir -p "$dir"
install -m 0755 "$tmp/whylog" "$dir/whylog"
echo "installed $dir/whylog ($("$dir/whylog" version))"
case ":$PATH:" in *":$dir:"*) ;; *) echo "note: add $dir to your PATH" ;; esac
echo "agents: run 'whylog skill --install' to add the Claude Code skill"
