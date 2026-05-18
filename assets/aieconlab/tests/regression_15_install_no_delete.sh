#!/usr/bin/env bash
# Regression for #15: dogfood install must not delete tracked AEL files.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE_HEAD="$(git -C "$REPO_ROOT" rev-parse HEAD)"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

fail() {
  echo "::error::$1" >&2
  exit 1
}

command -v git >/dev/null 2>&1 || fail "git is required"
command -v aiplus >/dev/null 2>&1 || fail "aiplus is required"

CLONE_DIR="$TMP_DIR/AiEconLab"
export HOME="$TMP_DIR/home"
export XDG_CONFIG_HOME="$HOME/.config"
mkdir -p "$HOME" "$XDG_CONFIG_HOME"

echo "$ git clone $REPO_ROOT $CLONE_DIR"
git clone --quiet --no-hardlinks "$REPO_ROOT" "$CLONE_DIR"
git -C "$CLONE_DIR" checkout --quiet "$SOURCE_HEAD"

cd "$CLONE_DIR"

git ls-files --error-unmatch acceptance/v0.1.0/schema.yaml >/dev/null \
  || fail "acceptance schema is not tracked at acceptance/v0.1.0/schema.yaml"
[ -f acceptance/v0.1.0/schema.yaml ] \
  || fail "acceptance schema missing before dogfood install"

echo "$ aiplus install codex --yes"
aiplus install codex --yes

echo "$ aiplus add aieconlab"
aiplus add aieconlab

echo "$ git status --short"
git status --short

deleted_count="$(git status --short | awk '$1 == "D" { count++ } END { print count + 0 }')"
[ "$deleted_count" -eq 0 ] \
  || fail "dogfood install deleted $deleted_count tracked file(s)"

[ -f acceptance/v0.1.0/schema.yaml ] \
  || fail "acceptance schema missing after dogfood install"

echo "PASS: #15 dogfood install preserved tracked files"
