#!/usr/bin/env bash
# Build and optionally package the AEL independent wrapper with its vendored runtime.

set -euo pipefail

AEL_VERSION="${AEL_VERSION:-0.1.4}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VENDOR_ROOT="$REPO_ROOT/vendor/aiplus"
DIST_ROOT="$REPO_ROOT/dist"

usage() {
  cat <<'EOF'
Usage:
  scripts/build-ael.sh [--package]

Builds vendor/aiplus/target/release/aiplus after syncing this AEL checkout into
the vendored runtime's bundled AEL asset. With --package, creates a release
tarball under dist/.
EOF
}

PACKAGE=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --package)
      PACKAGE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

[ -e "$VENDOR_ROOT/.git" ] || {
  echo "vendor/aiplus submodule is missing. Run: git submodule update --init --recursive" >&2
  exit 1
}

pre_sync_dirty="$(git -C "$VENDOR_ROOT" status --porcelain -- assets/aieconlab || true)"
if [ -n "$pre_sync_dirty" ]; then
  echo "vendor/aiplus/assets/aieconlab has pre-existing changes; refusing to overwrite them." >&2
  echo "$pre_sync_dirty" >&2
  exit 1
fi

cleanup_synced_asset() {
  git -C "$VENDOR_ROOT" restore --worktree --staged assets/aieconlab >/dev/null 2>&1 || true
  git -C "$VENDOR_ROOT" clean -fd -- assets/aieconlab >/dev/null 2>&1 || true
}
trap cleanup_synced_asset EXIT

sync_ael_asset() {
  mkdir -p "$VENDOR_ROOT/assets/aieconlab"
  rsync -a --delete \
    --exclude='.git' \
    --exclude='.github' \
    --exclude='vendor' \
    --exclude='dist' \
    --exclude='target' \
    --exclude='*.gif' \
    --exclude='*.png' \
    --exclude='*.jpg' \
    --exclude='*.mp4' \
    --exclude='*.mov' \
    "$REPO_ROOT/" "$VENDOR_ROOT/assets/aieconlab/"
}

host_triple() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
  esac
  printf '%s-%s\n' "$os" "$arch"
}

sync_ael_asset
cargo build --release --bin aiplus --manifest-path "$VENDOR_ROOT/Cargo.toml"
"$VENDOR_ROOT/target/release/aiplus" --version

if [ "$PACKAGE" -eq 1 ]; then
  triple="$(host_triple)"
  package_dir="$DIST_ROOT/ael-v$AEL_VERSION-$triple"
  rm -rf "$package_dir"
  mkdir -p "$package_dir/bin" "$package_dir/libexec"
  cp "$REPO_ROOT/ael" "$package_dir/bin/ael"
  cp "$VENDOR_ROOT/target/release/aiplus" "$package_dir/libexec/ael-support"
  cp "$REPO_ROOT/LICENSE" "$package_dir/LICENSE"
  cp "$REPO_ROOT/README.md" "$package_dir/README.md"
  chmod +x "$package_dir/bin/ael" "$package_dir/libexec/ael-support"
  tar -C "$DIST_ROOT" -czf "$package_dir.tar.gz" "$(basename "$package_dir")"
  echo "AEL_PACKAGE=$package_dir.tar.gz"
else
  echo "AEL_BUILD=PASS"
fi
