#!/usr/bin/env bash
set -euo pipefail

readonly KINDLE=kindle
readonly SSH=(ssh -o BatchMode=yes -o LogLevel=ERROR -o ConnectTimeout=5)
readonly RECORD="${XDG_STATE_HOME:-$HOME/.local/state}/omarchy/agents/usage/claude.json"
readonly REMOTE=/mnt/us/kindle-dev/usage
readonly PORT_SIDE="${KINDLE_PORT:-right}"

refresh=()
if [ "${1:-}" = "--flash" ] || [ "$(date +%-M)" -lt 5 ]; then
    refresh=(--flash)
fi
"${SSH[@]}" "$KINDLE" "$REMOTE" --now "$(date --iso-8601=seconds)" --port "$PORT_SIDE" "${refresh[@]}" < "$RECORD"
