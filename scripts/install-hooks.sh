#!/bin/bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
source_hook="$repo_root/scripts/hooks/pre-commit"
git_common_dir=$(git -C "$repo_root" rev-parse --git-common-dir)
case "$git_common_dir" in
  /*) ;;
  *) git_common_dir="$repo_root/$git_common_dir" ;;
esac
target_hook="$git_common_dir/hooks/pre-commit"
force=0

if [ "${1:-}" = "--force" ]; then
  force=1
fi

if [ ! -f "$source_hook" ]; then
  echo "ERROR: missing hook template: $source_hook" >&2
  exit 1
fi

mkdir -p "$(dirname "$target_hook")"

if [ -f "$target_hook" ] && ! cmp -s "$source_hook" "$target_hook" && [ "$force" -ne 1 ]; then
  echo "ERROR: existing pre-commit hook differs from scripts/hooks/pre-commit" >&2
  echo "Re-run with --force to replace it." >&2
  exit 1
fi

cp "$source_hook" "$target_hook"
chmod +x "$target_hook"
echo "Installed pre-commit hook: $target_hook"
