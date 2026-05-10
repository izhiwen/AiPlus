#!/usr/bin/env bash
# artifact-check.sh
# Dry-run artifact build and verification
# Local-only, temp staging, no upload
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TEMP_DIR="$(mktemp -d /tmp/aiplus-artifact-check-XXXXXX)"
trap 'rm -rf "${TEMP_DIR}"' EXIT

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

section "1. Build Release Binary"

echo "Building release binary (this may take a while)..."
if cargo build --release -p aiplus-cli 2>/dev/null; then
    pass "Release binary built"
    BINARY_PATH="${PROJECT_ROOT}/target/release/aiplus"
    if [[ -f "${BINARY_PATH}" ]]; then
        BINARY_SIZE=$(du -h "${BINARY_PATH}" | cut -f1)
        pass "Binary exists: ${BINARY_SIZE}"
    else
        block "Binary not found at expected path"
    fi
else
    warn "Release build failed (may need dependencies)"
    BINARY_PATH=""
fi

section "2. Create Temp Archive"

if [[ -n "${BINARY_PATH}" && -f "${BINARY_PATH}" ]]; then
    ARCHIVE_NAME="aiplus-$(grep -E '^version' crates/aiplus-cli/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')-aarch64-apple-darwin.tar.gz"
    ARCHIVE_PATH="${TEMP_DIR}/${ARCHIVE_NAME}"
    
    # Create archive with required files
    # Copy binary to temp dir to avoid --exclude='target' excluding it
    cp "${BINARY_PATH}" "${TEMP_DIR}/aiplus"
    cp "${PROJECT_ROOT}/LICENSE" "${TEMP_DIR}/LICENSE"
    cp "${PROJECT_ROOT}/README.md" "${TEMP_DIR}/README.md"
    cp "${PROJECT_ROOT}/install.sh" "${TEMP_DIR}/install.sh"
    tar -czf "${ARCHIVE_PATH}" \
        -C "${TEMP_DIR}" \
        aiplus \
        LICENSE \
        README.md \
        install.sh \
        2>/dev/null || true
    
    if [[ -f "${ARCHIVE_PATH}" ]]; then
        ARCHIVE_SIZE=$(du -h "${ARCHIVE_PATH}" | cut -f1)
        pass "Archive created: ${ARCHIVE_SIZE}"
        
        # List archive contents
        echo ""
        echo "Archive contents:"
        tar -tzf "${ARCHIVE_PATH}" | head -20
        
        # Check for excluded items
        EXCLUDED_FOUND=$(tar -tzf "${ARCHIVE_PATH}" | grep -E "\.env|secret|private|checkpoint|transcript|\.log$" || true)
        if [[ -n "${EXCLUDED_FOUND}" ]]; then
            warn "Potentially excluded items found in archive:"
            echo "${EXCLUDED_FOUND}" | head -10
        else
            pass "No excluded items in archive"
        fi
        
        # Generate checksum
        CHECKSUM=$(shasum -a 256 "${ARCHIVE_PATH}" | cut -d' ' -f1)
        echo ""
        echo "Checksum (SHA-256): ${CHECKSUM}"
        pass "Checksum generated"
        
        # Verify LICENSE is included
        if tar -tzf "${ARCHIVE_PATH}" | grep -q "LICENSE"; then
            pass "LICENSE included in archive"
        else
            block "LICENSE missing from archive"
        fi
        
        # Verify binary is included
        if tar -tzf "${ARCHIVE_PATH}" | grep -q "aiplus$"; then
            pass "Binary included in archive"
        else
            block "Binary missing from archive"
        fi
    else
        warn "Archive creation failed"
    fi
else
    warn "Skipping archive creation (binary not available)"
fi

section "3. Check Existing Release Artifacts"

if [[ -d "${PROJECT_ROOT}/release-artifacts" ]]; then
    ARTIFACT_COUNT=$(find "${PROJECT_ROOT}/release-artifacts" -type f | wc -l | tr -d ' ')
    echo "Existing artifacts: ${ARTIFACT_COUNT} files"
    
    if [[ ${ARTIFACT_COUNT} -gt 0 ]]; then
        echo ""
        echo "Artifact listing:"
        ls -lh "${PROJECT_ROOT}/release-artifacts/" | tail -n +2
        pass "Release artifacts directory exists"
    fi
else
    warn "No release-artifacts directory found"
fi

section "4. Verify No Private Content in Staging"

PRIVATE_CHECK=$(find "${PROJECT_ROOT}" -maxdepth 2 -not -path "*/scripts/*" -name "*zhiwen*" -o -not -path "*/scripts/*" -name "*private*" -o -not -path "*/scripts/*" -name "*secret*" 2>/dev/null | grep -v "secret-aliases" | grep -v ".git" | grep -v "scripts/" | head -10 || true)
if [[ -n "${PRIVATE_CHECK}" ]]; then
    warn "Potential private files in project root:"
    echo "${PRIVATE_CHECK}"
else
    pass "No private files in project root"
fi

section "5. Summary"

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
