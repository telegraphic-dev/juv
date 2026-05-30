#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

git diff --check
generator_diff_before="$(mktemp)"
generator_diff_after="$(mktemp)"
trap 'rm -f "$generator_diff_before" "$generator_diff_after"' EXIT
git diff --binary -- website/content/pages/docs/commands skill-data skills > "$generator_diff_before"
python3 scripts/generate-agent-docs.py
git diff --binary -- website/content/pages/docs/commands skill-data skills > "$generator_diff_after"
if ! cmp -s "$generator_diff_before" "$generator_diff_after"; then
  printf 'Generated command docs/skills are out of date. Run scripts/generate-agent-docs.py and commit the result.\n' >&2
  git diff --stat -- website/content/pages/docs/commands skill-data skills >&2
  exit 1
fi
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

unexpected_static_skills=$(find skills -mindepth 1 -maxdepth 1 -type d ! -name jbx -print | sort)
if [[ -n "$unexpected_static_skills" ]]; then
  printf 'Only skills/jbx is allowed as a public discovery stub; command skills must be served from the jbx binary. Unexpected static skills:\n%s\n' "$unexpected_static_skills" >&2
  exit 1
fi
