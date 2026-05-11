#!/usr/bin/env bash
# subproduct-drift.sh
# Compare bundled assets in aiplus-public against sibling subproducts
# Expected differences documented; unexpected differences flagged
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
AIPLUS_ROOT="$(cd "${PROJECT_ROOT}/.." && pwd)"

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

compare_subproduct() {
    local name="$1"
    local asset_path="${PROJECT_ROOT}/assets/${name}"
    local source_path="${AIPLUS_ROOT}/${name}"
    
    section "Comparing ${name}"
    
    if [[ ! -d "${asset_path}" ]]; then
        warn "Asset path not found: ${asset_path}"
        return
    fi
    
    if [[ ! -d "${source_path}" ]]; then
        warn "Source path not found: ${source_path}"
        return
    fi
    
    # Quick file count comparison
    local asset_count=$(find "${asset_path}" -type f | wc -l | tr -d ' ')
    local source_count=$(find "${source_path}" -type f -not -path "*/.git/*" | wc -l | tr -d ' ')
    
    echo "Asset files: ${asset_count}"
    echo "Source files: ${source_count}"
    
    # Check for critical files in assets
    local critical_missing=0
    for file in "README.md" "MODULES.md" "SECURITY.md"; do
        if [[ ! -f "${asset_path}/${file}" ]]; then
            warn "Missing ${file} in asset bundle"
            ((critical_missing++)) || true
        fi
    done
    
    if [[ ${critical_missing} -eq 0 ]]; then
        pass "Critical files present in asset bundle"
    fi
    
    # Check for files that should NOT be in assets (private content)
    # Exclude normal public files like CHANGELOG, README, MODULES from false positive grep
    # Exclude example files that intentionally show blocked/redacted content
    local private_leak=$(find "${asset_path}" -type f \
        -not -name "CHANGELOG.md" \
        -not -name "README.md" \
        -not -name "README.zh-CN.md" \
        -not -name "MODULES.md" \
        -not -name "aiplus-module.json" \
        -not -name "RELEASE_CHECKLIST.md" \
        -not -name "*.schema.json" \
        -not -name "*.md" \
        -not -name "*.example*" \
        -not -name "*example*" \
        -not -name "*.blocked-*" \
        -not -name "*.redacted*" \
        | xargs grep -ilE "aiplus-work-with-zhiwen.*content|work-with-zhiwen.*private|secret.*value|api_key|token.*sk-" 2>/dev/null || true)
    if [[ -n "${private_leak}" ]]; then
        block "Potential private content in asset bundle:"
        echo "${private_leak}" | head -5
    else
        pass "No private content detected in asset bundle"
    fi
    
    # Check version consistency if VERSION or version file exists
    if [[ -f "${source_path}/VERSION" ]]; then
        local source_version=$(cat "${source_path}/VERSION" | tr -d '[:space:]')
        if [[ -f "${asset_path}/VERSION" ]]; then
            local asset_version=$(cat "${asset_path}/VERSION" | tr -d '[:space:]')
            if [[ "${source_version}" == "${asset_version}" ]]; then
                pass "Version match: ${source_version}"
            else
                warn "Version mismatch: source=${source_version}, asset=${asset_version}"
            fi
        else
            warn "No VERSION file in asset bundle"
        fi
    fi
}

cd "${PROJECT_ROOT}"

section "Subproduct Drift Analysis"

compare_subproduct "aiplus-compact-reminder"
compare_subproduct "aiplus-auto-team-consultant"
compare_subproduct "aiplus-agent-memory"

section "Summary"

echo ""
echo "=========================================="
if [[ ${BLOCK_COUNT} -gt 0 ]]; then
    echo "STATUS: BLOCKED (${BLOCK_COUNT} blockers)"
    exit 1
elif [[ ${WARN_COUNT} -gt 0 ]]; then
    echo "STATUS: NEEDS_FIX (${WARN_COUNT} warnings)"
    exit 2
else
    echo "STATUS: PASS"
    exit 0
fi
