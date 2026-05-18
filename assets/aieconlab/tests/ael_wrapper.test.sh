#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

bash -n ael
bash -n scripts/build-ael.sh

version="$(./ael --version)"
[ "$version" = "AEL 0.1.4" ] || {
  echo "::error::unexpected ael version output: $version"
  exit 1
}

help="$(./ael --help)"
case "$help" in
  *AiPlus*|*aiplus*|*AIPLUS*)
    echo "::error::ael help leaks substrate branding"
    exit 1
    ;;
esac

dry_run="$(./ael install codex --dry-run)"
case "$dry_run" in
  *AiPlus*|*aiplus*|*AIPLUS*|*.AEL*)
    echo "::error::ael install dry-run leaks or corrupts substrate details"
    printf '%s\n' "$dry_run"
    exit 1
    ;;
esac

fake_bin="$(mktemp -d)"
cat >"$fake_bin/codex" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
answer_file=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-last-message)
      answer_file="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
[ -n "$answer_file" ] || exit 2
printf 'AiPlus leak via headless aiplus AIPLUS\n' >"$answer_file"
printf 'noisy AiPlus session log\n'
SH
chmod +x "$fake_bin/codex"
talk_output="$(PATH="$fake_bin:$PATH" ./ael talk --runtime codex advisor "What is your role?")"
case "$talk_output" in
  *AiPlus*|*aiplus*|*AIPLUS*)
    echo "::error::ael talk output leaks substrate branding"
    printf '%s\n' "$talk_output"
    exit 1
    ;;
esac

grep -q "vendor/aiplus/target/release" ael || {
  echo "::error::ael wrapper does not dispatch to vendored runtime"
  exit 1
}

grep -q "0.1.4" scripts/build-ael.sh || {
  echo "::error::build script missing v0.1.4 version anchor"
  exit 1
}

echo "AEL_WRAPPER_TEST=PASS"
