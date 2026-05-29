#!/usr/bin/env sh
set -eu

bin="jbx"
crate="jbx"
version="${JBX_VERSION:-latest}"
install_dir="${JBX_INSTALL_DIR:-$HOME/.jbx/bin}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "jbx installer: Rust/Cargo is required." >&2
  echo "Install Rust from https://rustup.rs/, then rerun this script." >&2
  exit 1
fi

mkdir -p "$install_dir"

tmp="$(mktemp -d 2>/dev/null || mktemp -d -t jbx-install)"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT INT TERM

if [ "$version" = "latest" ]; then
  cargo install "$crate" --locked --root "$tmp/root"
else
  cargo install "$crate" --version "${version#v}" --locked --root "$tmp/root"
fi

cp "$tmp/root/bin/$bin" "$install_dir/$bin"
chmod +x "$install_dir/$bin"

case ":$PATH:" in
  *":$install_dir:"*)
    echo "jbx installed: $($install_dir/$bin --version)"
    ;;
  *)
    echo "jbx installed to $install_dir/$bin"
    echo "Add this to your shell profile if it is not already on PATH:"
    echo "  export PATH=\"$install_dir:\$PATH\""
    ;;
esac
