#!/bin/sh
set -eu

REPO="izhiwen/aiplus"
# Pre-initialize VERSION so `set -eu` does not bomb when the `gh` branch
# is skipped (fresh Linux box without GitHub CLI). Fixes upstream issue
# izhiwen/AiPlus#1.
VERSION=""
if [ -n "${AIPLUS_VERSION:-}" ]; then
  VERSION="$AIPLUS_VERSION"
else
  if command -v gh >/dev/null 2>&1; then
    VERSION=$(gh api repos/izhiwen/aiplus/releases/latest --jq .tag_name 2>/dev/null || echo "")
  fi
  if [ -z "$VERSION" ] && command -v curl >/dev/null 2>&1; then
    VERSION=$(curl -fsSL https://api.github.com/repos/izhiwen/aiplus/releases/latest 2>/dev/null \
      | grep -m1 '"tag_name"' \
      | sed 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/' \
      || echo "")
  fi
  VERSION="${VERSION:-v0.5.8}"  # fallback if both lookups fail
fi
INSTALL_DIR="${AIPLUS_INSTALL_DIR:-$HOME/.local/bin}"
DRY_RUN=0

usage() {
  cat <<'USAGE'
Install the aiplus command.

Usage:
  sh install.sh [--dry-run]

Environment:
  AIPLUS_VERSION      Release version to install, default latest GitHub release
  AIPLUS_INSTALL_DIR  Install directory, default $HOME/.local/bin

The installer downloads a GitHub Release asset, verifies checksums.txt, and
installs only the aiplus binary. It does not edit shell profiles, require sudo,
install project modules, upload data, collect telemetry, or modify global
Codex/Claude Code/OpenCode config.

Supported platforms (auto-detected by uname):
  Darwin arm64 / aarch64           macOS Apple Silicon
  Darwin x86_64                    macOS Intel
  Linux x86_64                     Linux x86_64 (most CI runners, most servers)
  Linux aarch64 / arm64            Linux ARM64 (newer cloud, Docker on Apple Silicon)
  Windows x86_64                   Use install.ps1 instead of this script:
                                     iwr -useb https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.ps1 | iex
                                   (or run install.sh under WSL2, which is Linux x86_64).
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "ERROR required command not found: $1" >&2
    exit 1
  fi
}

detect_asset() {
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os:$arch" in
    Darwin:arm64|Darwin:aarch64)
      echo "aiplus-aarch64-apple-darwin.tar.gz"
      ;;
    Darwin:x86_64)
      echo "aiplus-x86_64-apple-darwin.tar.gz"
      ;;
    Linux:x86_64)
      echo "aiplus-x86_64-unknown-linux-gnu.tar.gz"
      ;;
    Linux:aarch64|Linux:arm64)
      echo "aiplus-aarch64-unknown-linux-gnu.tar.gz"
      ;;
    *)
      echo "ERROR no verified AiPlus $VERSION binary asset for: $os $arch" >&2
      echo "Supported platforms (v0.5.8+): Darwin arm64/x86_64, Linux x86_64/aarch64." >&2
      echo "Windows users: see Developer Build instructions until the Windows binary lands in v0.6.x." >&2
      echo "Source build fallback: clone https://github.com/$REPO and run 'cargo build --release -p aiplus-cli'." >&2
      exit 1
      ;;
  esac
}

sha256_verify() {
  checksums="$1"
  asset="$2"
  asset_name="$(basename "$asset")"
  expected="$(grep "  $asset_name\$" "$checksums" || true)"
  if [ -z "$expected" ]; then
    echo "ERROR checksum not found for $asset_name" >&2
    exit 1
  fi
  printf '%s\n' "$expected" > "$TMP_DIR/asset.sha256"
  if command -v shasum >/dev/null 2>&1; then
    (cd "$(dirname "$asset")" && shasum -a 256 -c "$TMP_DIR/asset.sha256")
  elif command -v sha256sum >/dev/null 2>&1; then
    (cd "$(dirname "$asset")" && sha256sum -c "$TMP_DIR/asset.sha256")
  else
    echo "ERROR shasum or sha256sum is required for checksum verification" >&2
    exit 1
  fi
}

need_cmd uname
need_cmd mktemp
need_cmd tar
need_cmd chmod

if command -v curl >/dev/null 2>&1; then
  fetch() {
    curl -fsSL "$1" -o "$2"
  }
elif command -v wget >/dev/null 2>&1; then
  fetch() {
    wget -q "$1" -O "$2"
  }
else
  echo "ERROR curl or wget is required" >&2
  exit 1
fi

ASSET="$(detect_asset)"
BASE_URL="https://github.com/$REPO/releases/download/$VERSION"
TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

echo "AiPlus installer"
echo "version=$VERSION"
echo "asset=$ASSET"
echo "install_dir=$INSTALL_DIR"
echo "writes=$INSTALL_DIR/aiplus"
echo "shell_profile_edits=none"
echo "telemetry=none"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "DRY_RUN=YES"
  echo "download=$BASE_URL/$ASSET"
  echo "checksums=$BASE_URL/checksums.txt"
  exit 0
fi

fetch "$BASE_URL/checksums.txt" "$TMP_DIR/checksums.txt"
fetch "$BASE_URL/$ASSET" "$TMP_DIR/$ASSET"
sha256_verify "$TMP_DIR/checksums.txt" "$TMP_DIR/$ASSET"

mkdir -p "$TMP_DIR/extract"
case "$ASSET" in
  *.tar.gz)
    tar -xzf "$TMP_DIR/$ASSET" -C "$TMP_DIR/extract"
    BIN="$TMP_DIR/extract/aiplus"
    if [ ! -f "$BIN" ]; then
      BIN="$(find "$TMP_DIR/extract" -type f -name aiplus | head -n 1)"
    fi
    ;;
  *.zip)
    need_cmd unzip
    unzip -q "$TMP_DIR/$ASSET" -d "$TMP_DIR/extract"
    BIN="$TMP_DIR/extract/aiplus.exe"
    if [ ! -f "$BIN" ]; then
      BIN="$(find "$TMP_DIR/extract" -type f -name aiplus.exe | head -n 1)"
    fi
    ;;
  *)
    echo "ERROR unsupported asset extension: $ASSET" >&2
    exit 1
    ;;
esac

if [ ! -f "$BIN" ]; then
  echo "ERROR release archive did not contain aiplus binary" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
chmod 755 "$BIN"
cp "$BIN" "$INSTALL_DIR/aiplus"
chmod 755 "$INSTALL_DIR/aiplus"

echo "INSTALL_STATUS=PASS"
echo "installed=$INSTALL_DIR/aiplus"

# Runtime dependency check (Linux only). The aiplus binary links against
# libdbus-1.so.3 because the `keyring` crate uses Linux Secret Service for
# secret-broker token persistence. Most desktop Linux systems already
# have it; minimal containers (e.g. ubuntu:22.04 base) do not. Warn the
# user instead of failing — install completed successfully either way.
if [ "$(uname -s)" = "Linux" ]; then
  has_libdbus=0
  if command -v ldconfig >/dev/null 2>&1; then
    if ldconfig -p 2>/dev/null | grep -q 'libdbus-1\.so\.3'; then
      has_libdbus=1
    fi
  fi
  if [ "$has_libdbus" -eq 0 ]; then
    echo ""
    echo "RUNTIME_DEP_NOTICE=libdbus-1.so.3 not found"
    echo "aiplus needs the libdbus-1 runtime library on Linux for secret storage."
    echo "Install it before running aiplus:"
    echo "  Debian/Ubuntu: sudo apt-get install -y libdbus-1-3"
    echo "  Fedora/RHEL:   sudo dnf install -y dbus-libs"
    echo "  Alpine:        sudo apk add dbus-libs"
  fi
fi

case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    ;;
  *)
    echo "PATH_NOTICE=$INSTALL_DIR is not on PATH"
    echo "Add this to your shell profile if you want to run aiplus from any terminal:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac

echo "Next:"
echo "  cd MyProject"
echo "  aiplus install codex"
