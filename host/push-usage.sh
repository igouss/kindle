#!/usr/bin/env bash
set -euo pipefail

readonly KINDLE=kindle
readonly SSH=(ssh -o BatchMode=yes -o LogLevel=ERROR -o ConnectTimeout=5)
readonly RECORD="${XDG_STATE_HOME:-$HOME/.local/state}/omarchy/agents/usage/claude.json"
readonly REMOTE=/mnt/us/kindle-dev/usage
readonly HERE="$(dirname "$(realpath "$0")")"

refresh=()
if [ "${1:-}" = "--flash" ] || [ "$(date +%-M)" -lt 5 ]; then
    refresh=(--flash)
fi
days="$("$HERE/tokens-by-day.sh")"
jq -e --argjson days "$days" '.recentDays = $days' "$RECORD" |
    "${SSH[@]}" "$KINDLE" "$REMOTE" push --now "$(date --iso-8601=seconds)" "${refresh[@]}"
