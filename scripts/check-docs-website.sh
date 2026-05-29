#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

git diff --check
npm --prefix website run check
npm --prefix website run build

if [[ -f website/public/install.sh ]]; then
  bash -n website/public/install.sh
fi
