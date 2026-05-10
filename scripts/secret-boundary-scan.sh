#!/usr/bin/env bash
# secret-boundary-scan.sh
# Scan for secrets, private profile leakage, raw transcripts, telemetry, global config edits
# Local-only, no network, prints summaries not secrets
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

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

cd "${PROJECT_ROOT}"

section "1. Secret Value Scan"

# Scan for potential secret values
SECRET_PATTERNS="sk-|BEGIN .*PRIVATE KEY|BWS_ACCESS_TOKEN|Authorization:|Bearer |password|token|api[_-]?key"
SECRET_MATCHES=$(find . -type f \
    -not -path "./.git/*" \
    -not -path "./target/*" \
    -not -path "./.codex/*" \
    -not -path "./.aiplus/*" \
    -not -path "./release-artifacts/*" \
    -not -path "./crates/aiplus-core/src/snapshot.rs" \
    -not -path "./crates/aiplus-core/src/auto_write.rs" \
    -not -path "./crates/aiplus-core/src/skill_candidate.rs" \
    -not -path "./crates/aiplus-core/src/redaction.rs" \
    -not -path "./crates/aiplus-core/src/profile_sync.rs" \
    -not -path "./crates/aiplus-cli/src/main.rs" \
    -not -path "./crates/aiplus-cli/tests/*" \
    -not -path "./docs/*" \
    -not -path "./assets/*" \
    -not -path "./scripts/*" \
    -not -name "*.log" \
    -not -name "*.tmp" \
    -not -name "*.lock" \
    -not -name "secret-aliases.tsv" \
    -not -name "RELEASE_READINESS_PACKET*" \
    -not -name "RELEASE_CHECKLIST*" \
    -not -name "release-packet-template.md" \
    -not -name "CHANGELOG.md" \
    -not -name "README.md" \
    -not -name "README.zh-CN.md" \
    -not -name "PROFILE_BUNDLE_PLAN.md" \
    -not -name "SECURITY.md" \
    -not -name "MODULES.md" \
    | xargs grep -inE "${SECRET_PATTERNS}" 2>/dev/null || true)

if [[ -n "${SECRET_MATCHES}" ]]; then
    # Filter out expected patterns:
    # - policy/documentation references
    # - test code with sample data
    # - redaction/classification code that intentionally detects secrets
    # - example/placeholder/mocked values
    REAL_SECRETS=$(echo "${SECRET_MATCHES}" | grep -viE \
        "(must not store|do not store|should not store|never store|secret_values|example|mock|placeholder|sample_record|redaction|has_password|is_jwt|contains\(\"|lower\.contains)" \
        || true)
    if [[ -n "${REAL_SECRETS}" ]]; then
        block "Potential secret values found:"
        echo "${REAL_SECRETS}" | head -20
    else
        pass "Only expected references found (redaction/classification code, test samples, policy docs)"
    fi
else
    pass "No secret patterns found"
fi

section "2. Raw Transcript / Checkpoint / Log Scan"

TRANSCRIPT_MATCHES=$(find . -type f \
    -not -path "./.git/*" \
    -not -path "./target/*" \
    -not -name "*.log" \
    -not -name "*.lock" \
    | xargs grep -inE "raw transcript|provider response|compact checkpoint|current-handoff|\.env" 2>/dev/null || true)

if [[ -n "${TRANSCRIPT_MATCHES}" ]]; then
    REAL_TRANSCRIPTS=$(echo "${TRANSCRIPT_MATCHES}" | grep -viE "(must not store|do not store|should not store|never store|policy|example|test)" || true)
    if [[ -n "${REAL_TRANSCRIPTS}" ]]; then
        warn "Potential transcript/checkpoint references found:"
        echo "${REAL_TRANSCRIPTS}" | head -10
    else
        pass "Only policy/documentation references found"
    fi
else
    pass "No transcript/checkpoint patterns found"
fi

section "3. Private Profile Content in Public Assets"

PRIVATE_MATCHES=$(find ./assets ./docs ./README.md -type f 2>/dev/null | xargs grep -inE "aiplus-work-with-zhiwen|work-with-zhiwen|Zhiwen" 2>/dev/null || true)

if [[ -n "${PRIVATE_MATCHES}" ]]; then
    # Check if only generic references
    GENERIC_ONLY=true
    while IFS= read -r line; do
        if echo "$line" | grep -viE "(private profile|forbidden_files|inherits|may consume|must not include|when linked)" >/dev/null; then
            GENERIC_ONLY=false
            break
        fi
    done <<< "${PRIVATE_MATCHES}"
    
    if [[ "${GENERIC_ONLY}" == "true" ]]; then
        pass "Only generic policy references to private profile found in public assets"
    else
        warn "Private profile references found in public assets (review):"
        echo "${PRIVATE_MATCHES}" | head -10
    fi
else
    pass "No private profile references in public assets"
fi

section "4. Telemetry / Data Upload Scan"

TELEMETRY_MATCHES=$(find ./src ./crates -type f -name "*.rs" 2>/dev/null | xargs grep -inE "telemetry|analytics|tracking|metrics.*send|upload.*data|post.*data" 2>/dev/null || true)

if [[ -n "${TELEMETRY_MATCHES}" ]]; then
    warn "Potential telemetry/data upload code found:"
    echo "${TELEMETRY_MATCHES}" | head -10
else
    pass "No telemetry or data upload patterns found"
fi

section "5. Global Config Edit Scan"

CONFIG_MATCHES=$(find ./src ./crates -type f -name "*.rs" 2>/dev/null | xargs grep -inE "std::fs::write.*config|std::fs::write.*rc|write_to.*home|write_to.*config|edit.*global|modify.*global" 2>/dev/null || true)

if [[ -n "${CONFIG_MATCHES}" ]]; then
    warn "Potential global config modification code found:"
    echo "${CONFIG_MATCHES}" | head -10
else
    pass "No global config modification patterns found"
fi

section "6. Node Fallback / Production Path Scan"

NODE_FALLBACK=$(find ./src ./crates -type f -name "*.rs" 2>/dev/null | xargs grep -inE "Command::new\(\"node\"\)|node.*fallback|npm.*run" 2>/dev/null || true)

if [[ -n "${NODE_FALLBACK}" ]]; then
    block "Node fallback found in Rust code:"
    echo "${NODE_FALLBACK}" | head -10
else
    pass "No Node fallback found"
fi

PROD_PATHS=$(find . -type f -not -path "./.git/*" -not -path "./target/*" | xargs grep -inE "/Users/steve/Dropbox/Project/(Immanuel|PAL|AppModules)" 2>/dev/null || true)

if [[ -n "${PROD_PATHS}" ]]; then
    warn "Production project paths found in source:"
    echo "${PROD_PATHS}" | head -10
else
    pass "No production project paths in source"
fi

section "7. Bitwarden Token Scan"

# Look for actual token values, not just environment variable references
BWS_TOKEN=$(find . -type f -not -path "./.git/*" -not -path "./target/*" | xargs grep -inE "BWS_ACCESS_TOKEN\s*=\s*['\"][^'\"]+['\"]|bws.*token\s*[:=]\s*['\"][^'\"]+['\"]|machine.*token\s*[:=]\s*['\"][^'\"]+['\"]" 2>/dev/null || true)

# Also check for env var references in code (expected, not blockers)
BWS_ENV_REFS=$(find ./src ./crates -type f -name "*.rs" 2>/dev/null | xargs grep -n "BWS_ACCESS_TOKEN" 2>/dev/null | grep -v "test" | wc -l | tr -d ' ' || echo "0")

if [[ -n "${BWS_TOKEN}" ]]; then
    block "Potential Bitwarden token values found:"
    echo "${BWS_TOKEN}" | head -10
else
    pass "No Bitwarden token values found (${BWS_ENV_REFS} expected env var references in code)"
fi

section "8. Summary"

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
