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
  VERSION="${VERSION:-v0.5.22}"  # fallback if both lookups fail (last-known-good)
fi
INSTALL_DIR="${AIPLUS_INSTALL_DIR:-$HOME/.local/bin}"
DRY_RUN=0
# P1.7: optional auto-register MCP server with installed runtimes.
# Values: "" (interactive prompt if tty), "yes" (silent register),
# "no" (silent skip).
REGISTER_MCP="${AIPLUS_REGISTER_MCP:-}"

usage() {
  cat <<'USAGE'
Install the aiplus command.

Usage:
  sh install.sh [--dry-run] [--register-mcp | --no-register-mcp]

Environment:
  AIPLUS_VERSION       Release version to install, default latest GitHub release
  AIPLUS_INSTALL_DIR   Install directory, default $HOME/.local/bin
  AIPLUS_REGISTER_MCP  "yes" / "no" — same as flags, but settable from CI etc.

Flags:
  --dry-run             Print what would happen without writing
  --register-mcp        After install, run `aiplus mcp-register` for any
                        detected runtime (codex / claude / opencode)
  --no-register-mcp     Skip the MCP registration prompt entirely
  -h, --help            Show this help

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
    --register-mcp)
      REGISTER_MCP=yes
      ;;
    --no-register-mcp)
      REGISTER_MCP=no
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

# Optional-feature notice (Linux only): the aiplus binary statically links
# libdbus (vendored in v0.5.11+), so it RUNS fine on any Linux. But to
# actually use the OS keyring for secret-broker token storage, a D-Bus
# session bus + a Secret Service daemon (gnome-keyring, kwallet, or
# pass-secret-service) must be available at runtime. Headless servers /
# minimal containers typically lack both. Tell the user about the
# BWS_ACCESS_TOKEN fallback up-front — install completed successfully
# either way.
if [ "$(uname -s)" = "Linux" ]; then
  if [ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ] && [ ! -S "/run/user/$(id -u 2>/dev/null)/bus" ]; then
    echo ""
    echo "OPTIONAL_NOTICE=no D-Bus session bus detected"
    echo "aiplus runs fine here. To use OS keyring storage for secret-broker"
    echo "tokens, you would need a D-Bus session bus + a Secret Service daemon"
    echo "(gnome-keyring / kwallet / pass-secret-service). For headless /"
    echo "container use, set BWS_ACCESS_TOKEN as an environment variable"
    echo "instead — keyring is optional, not required."
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

# ─────────────────────────────────────────────────────────────────────
# P1.7: MCP auto-registration. If we detect codex / claude-code /
# opencode installed (via their config dirs in $HOME), offer to run
# `aiplus mcp-register` for the user — this is the step that makes the
# MCP tools (agent_route, agent_status, etc.) callable from those
# runtimes. Without it, the binary install is "ready" but the agent
# workflow won't actually fire.
# ─────────────────────────────────────────────────────────────────────
detect_runtime_count() {
  count=0
  [ -d "$HOME/.codex" ] && count=$((count + 1))
  [ -d "$HOME/.claude" ] && count=$((count + 1))
  [ -d "$HOME/.opencode" ] && count=$((count + 1))
  echo "$count"
}

runtime_count=$(detect_runtime_count)
should_register=0
case "$REGISTER_MCP" in
  yes)
    should_register=1
    ;;
  no)
    should_register=0
    ;;
  "")
    # No explicit flag. Prompt only if we have a TTY and at least one
    # runtime is detected. Headless installs (CI, curl|bash piped to
    # bash without a tty) silently skip the prompt.
    if [ "$runtime_count" -gt 0 ] && [ -t 0 ] && [ -t 1 ]; then
      printf "Detected %d installed runtime(s) (codex/claude/opencode).\n" "$runtime_count"
      printf "Register aiplus MCP server with them now? This makes the agent_route\n"
      printf "and other PI tools callable from inside those runtimes. [Y/n] "
      read -r answer
      case "$answer" in
        n|N|no|NO) should_register=0 ;;
        *)         should_register=1 ;;
      esac
    fi
    ;;
esac

if [ "$should_register" = 1 ]; then
  echo ""
  echo "Running: $INSTALL_DIR/aiplus mcp-register"
  if "$INSTALL_DIR/aiplus" mcp-register; then
    echo "MCP_REGISTER_FROM_INSTALLER=OK"
  else
    echo "MCP_REGISTER_FROM_INSTALLER=FAIL — you can retry manually: aiplus mcp-register" >&2
  fi
elif [ "$runtime_count" -gt 0 ] && [ -z "$REGISTER_MCP" ]; then
  # Headless / non-tty install with detected runtimes — print the hint
  # so users know about mcp-register even when we can't prompt.
  echo ""
  echo "MCP_HINT detected $runtime_count runtime(s) but skipped the register prompt (no tty)"
  echo "Run \`aiplus mcp-register\` to enable agent_route + other MCP tools."
fi

echo "Next:"
echo "  cd MyProject"
echo "  aiplus install codex"
