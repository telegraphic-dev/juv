#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

git diff --check
npm --prefix website run check
npm --prefix website run build

if [[ -f website/public/install.sh ]]; then
  bash -n website/public/install.sh
fi

validate_skill_tree() {
  local root="$1"
  [[ -d "$root" ]] || return 0

  local found=0
  while IFS= read -r -d '' skill_dir; do
    found=1
    local skill_file="$skill_dir/SKILL.md"
    if [[ ! -f "$skill_file" ]]; then
      printf 'Missing %s\n' "$skill_file" >&2
      return 1
    fi

    python3 - "$skill_file" <<'PY'
from pathlib import Path
import re
import sys

path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
if not text.startswith("---\n"):
    raise SystemExit(f"{path}: missing YAML frontmatter")
try:
    _, frontmatter, body = text.split("---", 2)
except ValueError:
    raise SystemExit(f"{path}: malformed YAML frontmatter")
for field in ("name", "description"):
    if not re.search(rf"(?m)^{field}:\s*\S", frontmatter):
        raise SystemExit(f"{path}: missing frontmatter field {field}")
if not re.search(r"(?m)^#\s+\S", body):
    raise SystemExit(f"{path}: missing markdown heading")
PY
  done < <(find "$root" -mindepth 1 -maxdepth 1 -type d -print0 | sort -z)

  if [[ "$found" -eq 0 ]]; then
    printf 'No skill directories found under %s\n' "$root" >&2
    return 1
  fi
}

validate_skill_tree skill-data
validate_skill_tree skills
