#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

bash -n install.sh

dry_run="$(sh install.sh --dry-run)"
case "$dry_run" in
  *"version=v0.1.4"*) ;;
  *)
    echo "::error::install.sh default version must be v0.1.4"
    printf '%s\n' "$dry_run"
    exit 1
    ;;
esac
case "$dry_run" in
  *AiPlus*|*aiplus*|*AIPLUS*)
    echo "::error::install.sh dry-run leaks substrate branding"
    printf '%s\n' "$dry_run"
    exit 1
    ;;
esac

tmp="$(mktemp -d)"
release_dir="$tmp/release"
package_root="$tmp/pkg/ael-v9.9.9-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"
mkdir -p "$release_dir" "$package_root/bin" "$package_root/libexec"

cat >"$package_root/bin/ael" <<'SH'
#!/usr/bin/env bash
echo "fake ael"
SH
cat >"$package_root/libexec/ael-support" <<'SH'
#!/usr/bin/env bash
echo "fake support"
SH
chmod +x "$package_root/bin/ael" "$package_root/libexec/ael-support"

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "$arch" in
  arm64|aarch64) arch="aarch64" ;;
  x86_64|amd64) arch="x86_64" ;;
esac
asset="ael-v9.9.9-$os-$arch.tar.gz"
tar -C "$tmp/pkg" -czf "$release_dir/$asset" "$(basename "$package_root")"
if command -v shasum >/dev/null 2>&1; then
  (cd "$release_dir" && shasum -a 256 "$asset" > "$asset.sha256")
else
  (cd "$release_dir" && sha256sum "$asset" > "$asset.sha256")
fi

install_root="$tmp/install"
output="$(
  AEL_VERSION=v9.9.9 \
  AEL_BASE_URL="file://$release_dir" \
  AEL_INSTALL_DIR="$install_root/bin" \
  AEL_LIBEXEC_DIR="$install_root/libexec" \
  sh install.sh
)"

case "$output" in
  *INSTALL_STATUS=PASS*) ;;
  *)
    echo "::error::install.sh did not report pass"
    printf '%s\n' "$output"
    exit 1
    ;;
esac

[ -x "$install_root/bin/ael" ] || {
  echo "::error::ael wrapper not installed"
  exit 1
}
[ -x "$install_root/libexec/ael-support" ] || {
  echo "::error::support binary not installed"
  exit 1
}

echo "AEL_INSTALL_SH_TEST=PASS"
