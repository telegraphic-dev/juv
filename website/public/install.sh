#!/usr/bin/env sh
set -eu

bin="jbx"
version="${JBX_VERSION:-latest}"
install_dir="${JBX_INSTALL_DIR:-$HOME/.jbx/bin}"
repo="telegraphic-dev/jbx"

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "jbx installer: required command not found: $1" >&2
    exit 1
  fi
}

case "$(uname -s)" in
  Linux) os="unknown-linux-gnu" ;;
  Darwin) os="apple-darwin" ;;
  *)
    echo "jbx installer: unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

case "$(uname -m)" in
  x86_64|amd64) arch="x86_64" ;;
  arm64|aarch64) arch="aarch64" ;;
  *)
    echo "jbx installer: unsupported architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

target="$arch-$os"
asset="jbx-$target.tar.gz"

if [ "$version" = "latest" ]; then
  url="https://github.com/$repo/releases/latest/download/$asset"
else
  tag="$version"
  case "$tag" in
    v*) ;;
    *) tag="v$tag" ;;
  esac
  url="https://github.com/$repo/releases/download/$tag/$asset"
fi

need tar
mkdir -p "$install_dir"

tmp="$(mktemp -d 2>/dev/null || mktemp -d -t jbx-install)"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT INT TERM

if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$url" -o "$tmp/$asset"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$tmp/$asset" "$url"
else
  echo "jbx installer: curl or wget is required." >&2
  exit 1
fi

tar -xzf "$tmp/$asset" -C "$tmp"
cp "$tmp/$bin" "$install_dir/$bin"
chmod +x "$install_dir/$bin"

echo "jbx installed: $($install_dir/$bin --version)"

case ":$PATH:" in
  *":$install_dir:"*)
    ;;
  *)
    echo
    echo "Add jbx to PATH for this terminal:"
    echo "  export PATH=\"$install_dir:\$PATH\""
    echo
    echo "Make it permanent by adding that line to your shell profile, for example:"
    echo "  echo 'export PATH=\"$install_dir:\$PATH\"' >> ~/.bashrc"
    echo "  echo 'export PATH=\"$install_dir:\$PATH\"' >> ~/.zshrc"
    ;;
esac
