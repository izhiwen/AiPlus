#!/bin/sh
set -eu

REPO="izhiwen/aiplus"
VERSION="${AIPLUS_VERSION:-v0.4.4}"
INSTALL_DIR="${AIPLUS_INSTALL_DIR:-$HOME/.local/bin}"
DRY_RUN=0

usage() {
  cat <<'USAGE'
Install the aiplus command.

Usage:
  sh install.sh [--dry-run]

Environment:
  AIPLUS_VERSION      Release version to install, default v0.4.4
  AIPLUS_INSTALL_DIR  Install directory, default $HOME/.local/bin

The installer downloads a GitHub Release asset, verifies checksums.txt, and
installs only the aiplus binary. It does not edit shell profiles, require sudo,
install project modules, upload data, collect telemetry, or modify global
Codex/Claude Code/OpenCode config.

AiPlus v0.4.4 publishes a verified macOS Apple Silicon asset first. Other
platforms should use the Developer Build instructions until their assets are
published and verified.
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
    *)
      echo "ERROR no verified AiPlus v0.4.4 binary asset for: $os $arch" >&2
      echo "Use the Developer Build instructions until this platform is published." >&2
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
