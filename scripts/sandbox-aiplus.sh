#!/usr/bin/env bash
# sandbox-aiplus.sh — wraps aiplus commands in a temp HOME
set -euo pipefail

export HOME="$(mktemp -d)"
export XDG_CONFIG_HOME="$HOME/.config"
mkdir -p "$XDG_CONFIG_HOME"

exec "$@"
