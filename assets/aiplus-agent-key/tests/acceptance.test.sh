#!/usr/bin/env bash
# AiPlus-Agent-Key — acceptance test for v0.1.0 schema.
# Exits 0 on pass, non-zero on first failure.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

fail() {
  echo "::error::$1" >&2
  exit 1
}

pass() {
  echo "PASS: $1"
}

toml_parse() {
  python3 - "$1" <<'PY'
import sys
try:
    import tomllib as toml
except ImportError:
    try:
        import tomli as toml
    except ImportError:
        path = sys.argv[1]
        with open(path, "r", encoding="utf-8") as fh:
            txt = fh.read()
        if not txt.strip():
            sys.exit(2)
        sys.exit(0)
toml.loads(open(sys.argv[1], "rb").read().decode("utf-8") if hasattr(toml, "loads") else open(sys.argv[1]).read())
PY
}

# ---------------------------------------------------------------------
# core: module_manifest_present
# ---------------------------------------------------------------------
manifest="aiplus-module.json"
[ -f "$manifest" ] || fail "missing module manifest: $manifest"
python3 -c "import json; json.load(open('$manifest'))" \
  || fail "JSON parse failure: $manifest"
name=$(python3 -c "import json; print(json.load(open('$manifest'))['name'])")
[ "$name" = "agent-key" ] || fail "manifest name is '$name', expected 'agent-key'"
for adapter in codex claude-code opencode; do
  grep -q "\"$adapter\"" "$manifest" \
    || fail "manifest missing adapter: $adapter"
done
pass "module_manifest_present (name=agent-key, 3 adapters)"

# ---------------------------------------------------------------------
# core: license, readme, design
# ---------------------------------------------------------------------
[ -f "LICENSE" ] || fail "missing LICENSE"
pass "license_present"

for f in README.md README.zh-CN.md; do
  [ -f "$f" ] || fail "missing $f"
  size=$(wc -c < "$f")
  [ "$size" -ge 5000 ] || fail "$f too small (<5000B): $size bytes"
done
pass "readme_present (en + zh, both >=5000B)"

[ -f "DESIGN.md" ] || fail "missing DESIGN.md"
size=$(wc -c < "DESIGN.md")
[ "$size" -ge 8000 ] || fail "DESIGN.md too small (<8000B): $size bytes"
pass "design_doc_present (>=8000B)"

# ---------------------------------------------------------------------
# core: example aliases TSV (real config schema) + conventions
# ---------------------------------------------------------------------
[ -f "core/example-aliases.tsv" ] || fail "missing core/example-aliases.tsv"
# Every non-empty line must have exactly 3 tab-separated columns.
awk -F'\t' 'NF != 3 { print NR": "NF" columns, expected 3 ("$0")"; exit 1 }' core/example-aliases.tsv \
  || fail "core/example-aliases.tsv has a row that is not 3 tab-separated columns"
pass "example_aliases_present (TSV, every row has 3 columns)"

[ -f "core/example-aliases.md" ] || fail "missing core/example-aliases.md (TSV format walkthrough)"
size=$(wc -c < "core/example-aliases.md")
[ "$size" -ge 800 ] || fail "core/example-aliases.md too small (<800B): $size bytes"
pass "example_aliases_walkthrough_present (>=800B)"

[ -f "core/alias-conventions.md" ] || fail "missing core/alias-conventions.md"
size=$(wc -c < "core/alias-conventions.md")
[ "$size" -ge 1500 ] || fail "core/alias-conventions.md too small (<1500B): $size bytes"
pass "alias_conventions_present (>=1500B)"

# ---------------------------------------------------------------------
# core: adapters + examples
# ---------------------------------------------------------------------
for adapter in codex claude-code opencode; do
  f="adapters/${adapter}/README.md"
  [ -f "$f" ] || fail "missing adapter README: $f"
done
pass "adapters_present (3 adapter READMEs)"

[ -f "examples/README.md" ] || fail "missing examples/README.md"
pass "examples_present"

# ---------------------------------------------------------------------
# safety: .gitignore blocks secrets
# ---------------------------------------------------------------------
[ -f ".gitignore" ] || fail "missing .gitignore"
for pattern in "*.env" "*.token" "*.key" "*.credentials"; do
  grep -q "^${pattern}$\|^${pattern}\b" .gitignore \
    || fail ".gitignore missing pattern: $pattern"
done
pass "gitignore_blocks_secrets (*.env, *.token, *.key, *.credentials)"

# ---------------------------------------------------------------------
# safety: no_real_secrets_heuristic
# ---------------------------------------------------------------------
# Scan all text files for substrings matching real-credential patterns.
# Exclude this test script itself (which lists the patterns) and any
# example placeholders that intentionally show a truncated form.
forbidden_match=$(
  find . -type f \( -name '*.md' -o -name '*.toml' -o -name '*.json' -o -name '*.yaml' -o -name '*.yml' -o -name '*.sh' \) \
    -not -path './.git/*' \
    -not -path './tests/acceptance.test.sh' \
    -print0 \
  | xargs -0 grep -EHn -e 'sk-[A-Za-z0-9]{40,}' \
                    -e 'sk_live_[A-Za-z0-9]{30,}' \
                    -e 'ghp_[A-Za-z0-9]{36}' \
                    -e 'github_pat_[A-Za-z0-9_]{50,}' \
    2>/dev/null \
  | grep -v 'sk-proj-\.\.\.' \
  | head -3 || true
)
if [ -n "$forbidden_match" ]; then
  echo "::error::Possible real secret detected:"
  echo "$forbidden_match"
  fail "no_real_secrets_heuristic"
fi
pass "no_real_secrets_heuristic (no real-secret patterns found)"

# ---------------------------------------------------------------------
# done
# ---------------------------------------------------------------------
echo ""
echo "All v0.1.0 acceptance invariants passed."
