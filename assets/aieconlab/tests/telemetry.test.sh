#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

telemetry_path="$tmp_dir/telemetry.json"

status="$(AEL_TELEMETRY_PATH="$telemetry_path" ./ael telemetry status)"
case "$status" in
  *"status: disabled"*"$telemetry_path"* )
    ;;
  *)
    echo "::error::default telemetry status should be disabled"
    printf '%s\n' "$status"
    exit 1
    ;;
esac

case "$status" in
  *AiPlus*|*aiplus*|*AIPLUS*)
    echo "::error::telemetry status leaks substrate branding"
    printf '%s\n' "$status"
    exit 1
    ;;
esac

enable_output="$(AEL_TELEMETRY_PATH="$telemetry_path" ./ael telemetry enable)"
case "$enable_output" in
  *"telemetry enabled"*"$telemetry_path"* )
    ;;
  *)
    echo "::error::telemetry enable output missing expected markers"
    printf '%s\n' "$enable_output"
    exit 1
    ;;
esac

python3 - "$telemetry_path" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
assert data["schema_version"] == "v0.2.1"
assert data["enabled"] is True
assert data["mode"] == "local-json"
assert data["hosted_endpoint"] is None
assert data["events_path"] == ".ael/telemetry-events.jsonl"
assert sorted(data["event_payload"]) == [
    "expert",
    "outcome",
    "task_class",
    "timestamp",
]
PY

enabled_status="$(AEL_TELEMETRY_PATH="$telemetry_path" ./ael telemetry)"
case "$enabled_status" in
  *"status: enabled"* )
    ;;
  *)
    echo "::error::telemetry status should be enabled after opt-in"
    printf '%s\n' "$enabled_status"
    exit 1
    ;;
esac

AEL_TELEMETRY_PATH="$telemetry_path" ./ael telemetry disable >/dev/null
python3 - "$telemetry_path" <<'PY'
import json
import sys
from pathlib import Path

data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert data["enabled"] is False
PY

echo "AEL_TELEMETRY_TEST=PASS"
