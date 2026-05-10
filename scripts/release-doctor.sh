#!/usr/bin/env bash
# release-doctor.sh
# Release readiness dry-run for AiPlus v0.5.x
# Local-only, no network, no push/tag/release/upload
# Hard STOP on real blockers; WARN on advisory items
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPORT_DIR="${PROJECT_ROOT}/docs/stabilization"
REPORT_FILE="${REPORT_DIR}/release-doctor-report.md"
TEMP_DIR="$(mktemp -d /tmp/aiplus-release-doctor-XXXXXX)"
trap 'rm -rf "${TEMP_DIR}"' EXIT

# Colors
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

WARN_COUNT=0
BLOCK_COUNT=0
PASS_COUNT=0

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
    ((WARN_COUNT++)) || true
}

block() {
    echo -e "${RED}[BLOCK]${NC} $1"
    ((BLOCK_COUNT++)) || true
}

pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((PASS_COUNT++)) || true
}

section() {
    echo ""
    echo "=========================================="
    echo "$1"
    echo "=========================================="
}

# Ensure report dir exists
mkdir -p "${REPORT_DIR}"

# Begin report
cat > "${REPORT_FILE}" << 'HEADER'
# Release Doctor Report

Generated: REPLACEMENT_TIMESTAMP
Mode: dry-run, local-only
Commands blocked: push, tag, release, upload, publish, deploy

HEADER

# Replace timestamp
sed -i '' "s/REPLACEMENT_TIMESTAMP/$(date -u +%Y-%m-%dT%H:%M:%SZ)/" "${REPORT_FILE}" 2>/dev/null || \
sed -i "s/REPLACEMENT_TIMESTAMP/$(date -u +%Y-%m-%dT%H:%M:%SZ)/" "${REPORT_FILE}"

append_report() {
    echo "$1" >> "${REPORT_FILE}"
}

section "1. Environment & Version Check"
cd "${PROJECT_ROOT}"
append_report "## 1. Environment & Version Check"

# Check Cargo.toml version
CARGO_VERSION=$(grep -E '^version\s*=' crates/aiplus-cli/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [[ -n "${CARGO_VERSION}" ]]; then
    pass "Cargo.toml version: ${CARGO_VERSION}"
    append_report "- PASS: Cargo.toml version = \`${CARGO_VERSION}\`"
else
    block "Cannot read version from crates/aiplus-cli/Cargo.toml"
    append_report "- BLOCK: Cannot read version from crates/aiplus-cli/Cargo.toml"
fi

# Check CLI --version (source build)
if cargo run -p aiplus-cli -- --version 2>/dev/null | grep -q "${CARGO_VERSION}"; then
    pass "CLI --version matches Cargo.toml"
    append_report "- PASS: CLI --version matches Cargo.toml"
else
    warn "CLI --version may not match (or build needed)"
    append_report "- WARN: CLI --version mismatch or build needed"
fi

# Check CHANGELOG mentions version
if grep -q "${CARGO_VERSION}" CHANGELOG.md; then
    pass "CHANGELOG.md mentions version ${CARGO_VERSION}"
    append_report "- PASS: CHANGELOG.md mentions version"
else
    warn "CHANGELOG.md does not mention version ${CARGO_VERSION}"
    append_report "- WARN: CHANGELOG.md missing version mention"
fi

section "2. Rust QA"
append_report ""
append_report "## 2. Rust QA"

# cargo fmt
if cargo fmt --all --check 2>/dev/null; then
    pass "cargo fmt --all --check"
    append_report "- PASS: cargo fmt --all --check"
else
    block "cargo fmt --all --check failed"
    append_report "- BLOCK: cargo fmt --all --check failed"
fi

# cargo clippy
if cargo clippy --workspace --all-targets --all-features -- -D warnings 2>/dev/null; then
    pass "cargo clippy --workspace --all-targets --all-features"
    append_report "- PASS: cargo clippy"
else
    warn "cargo clippy found issues (or not installed)"
    append_report "- WARN: cargo clippy issues"
fi

# cargo test
if cargo test --workspace 2>/dev/null; then
    pass "cargo test --workspace"
    append_report "- PASS: cargo test --workspace"
else
    warn "cargo test failed (some tests may be expected to fail in dry-run)"
    append_report "- WARN: cargo test had failures"
fi

# cargo metadata
if cargo metadata --format-version 1 --no-deps >/dev/null 2>&1; then
    pass "cargo metadata parses"
    append_report "- PASS: cargo metadata parses"
else
    block "cargo metadata failed"
    append_report "- BLOCK: cargo metadata failed"
fi

# git diff --check
if git diff --check 2>/dev/null; then
    pass "git diff --check"
    append_report "- PASS: git diff --check"
else
    warn "git diff --check found issues"
    append_report "- WARN: git diff --check issues"
fi

section "3. CLI Smoke Tests (source)"
append_report ""
append_report "## 3. CLI Smoke Tests"

SMOKE_COMMANDS=(
    "--help"
    "doctor"
    "memory doctor"
    "profile doctor aiplus-work-with-zhiwen"
    "status"
    "compact validate"
)

for cmd in "${SMOKE_COMMANDS[@]}"; do
    if cargo run -p aiplus-cli -- ${cmd} >/dev/null 2>&1; then
        pass "cargo run -p aiplus-cli -- ${cmd}"
        append_report "- PASS: \`cargo run -p aiplus-cli -- ${cmd}\`"
    else
        warn "cargo run -p aiplus-cli -- ${cmd} failed"
        append_report "- WARN: \`cargo run -p aiplus-cli -- ${cmd}\` failed"
    fi
done

section "4. Safety & Boundary Checks"
append_report ""
append_report "## 4. Safety & Boundary Checks"

# Check no push/tag/release/upload in scripts
# Only check for actual commands, not comments or documentation
PUSH_TAG_MATCHES=$(grep -rnE "(git push|git tag|github release|cargo publish|npm publish|brew .*upload|gh release)" scripts/ 2>/dev/null | grep -v "#.*\(no\|do not\|blocked\|refuse\|safety\|Local-only\)" | grep -v "warn.*push/tag/release/upload\|append_report.*push/tag/release/upload\|pass.*push/tag/release/upload" || true)
if [[ -n "${PUSH_TAG_MATCHES}" ]]; then
    warn "Scripts may contain push/tag/release/upload commands:"
    echo "${PUSH_TAG_MATCHES}" | head -5
    append_report "- WARN: Scripts contain push/tag/release/upload keywords (review required)"
else
    pass "No push/tag/release/upload commands in scripts"
    append_report "- PASS: No push/tag/release/upload commands in scripts"
fi

# Check LICENSE exists
if [[ -f "LICENSE" ]]; then
    pass "LICENSE exists"
    append_report "- PASS: LICENSE exists"
else
    block "LICENSE missing"
    append_report "- BLOCK: LICENSE missing"
fi

# Check publish = false in workspace
if grep -q "publish = false" Cargo.toml; then
    pass "Workspace has publish = false"
    append_report "- PASS: Workspace has publish = false"
else
    warn "Workspace may allow publishing"
    append_report "- WARN: Workspace may allow publishing"
fi

section "5. Report Summary"
append_report ""
append_report "## 5. Report Summary"
append_report ""
append_report "| Check | Count |"
append_report "|-------|-------|"
append_report "| PASS  | ${PASS_COUNT} |"
append_report "| WARN  | ${WARN_COUNT} |"
append_report "| BLOCK | ${BLOCK_COUNT} |"
append_report ""

if [[ ${BLOCK_COUNT} -gt 0 ]]; then
    append_report "**STATUS: BLOCKED** — ${BLOCK_COUNT} blocker(s) must be fixed."
    echo ""
    echo "=========================================="
    echo "STATUS: BLOCKED (${BLOCK_COUNT} blockers)"
    echo "=========================================="
    exit 1
elif [[ ${WARN_COUNT} -gt 0 ]]; then
    append_report "**STATUS: NEEDS_FIX** — ${WARN_COUNT} warning(s) to review."
    echo ""
    echo "=========================================="
    echo "STATUS: NEEDS_FIX (${WARN_COUNT} warnings)"
    echo "=========================================="
    exit 2
else
    append_report "**STATUS: PASS** — All checks passed."
    echo ""
    echo "=========================================="
    echo "STATUS: PASS"
    echo "=========================================="
    exit 0
fi
