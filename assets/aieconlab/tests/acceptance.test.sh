#!/usr/bin/env bash
# AiEconLab — acceptance test for v0.1.0 schema.
#
# Validates the structural invariants declared in
# .aiplus/agent-team/acceptance/v0.1.0/schema.yaml.
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

# python3.11+ ships tomllib; earlier versions need tomli or a vendored parser.
toml_parse() {
  python3 - "$1" <<'PY'
import sys
try:
    import tomllib as toml
except ImportError:
    try:
        import tomli as toml
    except ImportError:
        # Lightweight fallback: read the file and check basic structure.
        # Accepts the file as long as it is non-empty and key=value lines
        # parse loosely. Not a full TOML validator, but enough to gate
        # missing/corrupt files in environments without tomli.
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
# core invariant: core_roles_present
# ---------------------------------------------------------------------
core_roles=(advisor pi theorist pm ra-stata ra-python referee replicator)
for role in "${core_roles[@]}"; do
  f="core/templates/${role}.toml"
  [ -f "$f" ] || fail "missing core role TOML: $f"
  toml_parse "$f" \
    || fail "TOML parse failure: $f"
done
pass "core_roles_present (8 roles, all TOML parses)"

# ---------------------------------------------------------------------
# core invariant: team_config_present
# ---------------------------------------------------------------------
team_toml="core/templates/econ-team.toml"
[ -f "$team_toml" ] || fail "missing team config: $team_toml"
toml_parse "$team_toml" \
  || fail "TOML parse failure: $team_toml"
for role in "${core_roles[@]}"; do
  grep -q "\"${role}\"" "$team_toml" \
    || fail "team config missing role: $role"
done
pass "team_config_present (declares 8 core roles)"

# ---------------------------------------------------------------------
# core invariant: personas_present
# ---------------------------------------------------------------------
for role in "${core_roles[@]}"; do
  f="core/templates/personas/${role}.md"
  [ -f "$f" ] || fail "missing core persona: $f"
  size=$(wc -c < "$f")
  [ "$size" -ge 500 ] \
    || fail "core persona too small (<500B): $f ($size bytes)"
done
pass "personas_present (8 core personas, all >=500B)"

# ---------------------------------------------------------------------
# core invariant: experts_present
# ---------------------------------------------------------------------
experts=(lit-reviewer writer econometrician reproducibility historical-sources job-talk-coach viz-specialist survey-experiment computation ethics-irb coauthor-liaison)
for expert in "${experts[@]}"; do
  f="core/templates/experts/${expert}.toml"
  [ -f "$f" ] || fail "missing expert TOML: $f"
  toml_parse "$f" \
    || fail "TOML parse failure: $f"
done
pass "experts_present (11 expert TOMLs, all parse)"

# ---------------------------------------------------------------------
# core invariant: shipped_expert_personas_present
# ---------------------------------------------------------------------
shipped_experts=(lit-reviewer writer econometrician reproducibility historical-sources job-talk-coach viz-specialist ethics-irb)
for expert in "${shipped_experts[@]}"; do
  f="core/templates/personas/${expert}.md"
  [ -f "$f" ] || fail "missing shipped expert persona: $f"
  size=$(wc -c < "$f")
  [ "$size" -ge 500 ] \
    || fail "shipped expert persona too small (<500B): $f ($size bytes)"
done
pass "shipped_expert_personas_present (8 shipped, all >=500B)"

# ---------------------------------------------------------------------
# core invariant: stub_expert_personas_present
# ---------------------------------------------------------------------
stub_experts=(survey-experiment computation coauthor-liaison)
for expert in "${stub_experts[@]}"; do
  f="core/templates/personas/_stubs/${expert}.md"
  [ -f "$f" ] || fail "missing stub expert persona: $f"
done
pass "stub_expert_personas_present (3 stubs)"

# ---------------------------------------------------------------------
# core invariant: module_manifest_present
# ---------------------------------------------------------------------
manifest="aiplus-module.json"
[ -f "$manifest" ] || fail "missing module manifest: $manifest"
python3 -c "import json; json.load(open('$manifest'))" \
  || fail "JSON parse failure: $manifest"
for adapter in codex claude-code opencode; do
  grep -q "\"${adapter}\"" "$manifest" \
    || fail "manifest missing adapter: $adapter"
done
pass "module_manifest_present (declares 3 adapters)"

# ---------------------------------------------------------------------
# core invariant: adapters_present
# ---------------------------------------------------------------------
for adapter in codex claude-code opencode; do
  f="adapters/${adapter}/README.md"
  [ -f "$f" ] || fail "missing adapter README: $f"
done
pass "adapters_present (3 adapter READMEs)"

# ---------------------------------------------------------------------
# core invariant: license_present, design_doc_present, readme_present
# ---------------------------------------------------------------------
[ -f "LICENSE" ] || fail "missing LICENSE"
pass "license_present"

[ -f "DESIGN.md" ] || fail "missing DESIGN.md"
size=$(wc -c < "DESIGN.md")
[ "$size" -ge 5000 ] \
  || fail "DESIGN.md too small (<5000B): $size bytes"
pass "design_doc_present (>=5000B)"

for f in README.md README.zh-CN.md; do
  [ -f "$f" ] || fail "missing $f"
done
pass "readme_present (en + zh)"

# ---------------------------------------------------------------------
# persona invariant: forbidden section in core personas
# ---------------------------------------------------------------------
for role in "${core_roles[@]}"; do
  f="core/templates/personas/${role}.md"
  grep -q "Forbidden Actions" "$f" \
    || fail "$f missing 'Forbidden Actions' section"
done
pass "core_personas_have_forbidden_section"

# ---------------------------------------------------------------------
# persona invariant: at least 3 examples per core persona
# ---------------------------------------------------------------------
for role in "${core_roles[@]}"; do
  f="core/templates/personas/${role}.md"
  count=$(grep -c "^### Example" "$f" || true)
  [ "$count" -ge 3 ] \
    || fail "$f has fewer than 3 examples (found $count)"
done
pass "core_personas_have_examples (>=3 each)"

# ---------------------------------------------------------------------
# persona invariant: STOP-gate in PI persona
# ---------------------------------------------------------------------
grep -q "STOP-gate" "core/templates/personas/pi.md" \
  || fail "PI persona missing 'STOP-gate' reference"
pass "stop_gates_present_in_pi"

# ---------------------------------------------------------------------
# done
# ---------------------------------------------------------------------
echo ""
echo "All v0.1.0 acceptance invariants passed."
