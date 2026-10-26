#!/usr/bin/env bash
set -euo pipefail

readonly REMOTES=(fedora)
readonly SSH=(ssh -o BatchMode=yes -o LogLevel=ERROR -o ConnectTimeout=5)

tz="$(timedatectl show -p Timezone --value)"
dates=()
for back in 6 5 4 3 2 1 0; do
    dates+=("$(TZ="$tz" date -d "$back days ago" +%F)")
done
daily=(ccusage claude daily --json --offline --no-color --timezone "$tz" --since "${dates[0]//-/}")

{
    "${daily[@]}"
    for remote in "${REMOTES[@]}"; do
        "${SSH[@]}" "$remote" bash -lc "$(printf '%q' "$(printf '%q ' "${daily[@]}")")"
    done
} | jq -en '[inputs.daily[]] as $rows
    | $ARGS.positional
    | map(. as $date | {date: $date, messageCount: ([$rows[] | select(.date == $date) | .totalTokens] | add // 0)})' \
    --args "${dates[@]}"
