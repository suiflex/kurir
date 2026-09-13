#!/bin/sh
set -eu

version="${KURIR_VERSION:-0.1.0}"
install_dir="${KURIR_INSTALL_DIR:-$HOME/.local/bin}"
arch="$(uname -m)"
os="$(uname -s)"
case "$os-$arch" in
  Darwin-arm64|Darwin-aarch64) target="darwin-aarch64" ;;
  Darwin-x86_64) target="darwin-x86_64" ;;
  Linux-aarch64|Linux-arm64) target="linux-aarch64" ;;
  Linux-x86_64) target="linux-x86_64" ;;
  *) echo "Unsupported platform: $os-$arch" >&2; exit 1 ;;
esac

base="https://github.com/suiflex/kurir/releases/download/v${version}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL "$base/kurir-${version}-${target}.tar.gz" -o "$tmp/kurir.tar.gz"
tar -xzf "$tmp/kurir.tar.gz" -C "$tmp"
mkdir -p "$install_dir"
install "$tmp/kurir" "$install_dir/kurir"
printf 'Installed kurir %s to %s/kurir\n' "$version" "$install_dir"
