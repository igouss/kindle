#!/usr/bin/env bash
set -euo pipefail

readonly KINDLE=kindle
readonly SSH=(ssh -o BatchMode=yes -o LogLevel=ERROR)
readonly REMOTE_PATH="PATH=/usr/sbin:/sbin:/usr/bin:/bin"

name="${1:?usage: install-job.sh <job name in kindle/upstart, without .conf>}"
readonly JOB="$name.conf"
readonly LOCAL_JOB="$(dirname "$(readlink -f "$0")")/../kindle/upstart/$JOB"
[ -f "$LOCAL_JOB" ] || { echo "no such job: $LOCAL_JOB" >&2; exit 1; }

"${SSH[@]}" "$KINDLE" "$REMOTE_PATH; mntroot rw && cat > /etc/upstart/$JOB && mntroot ro" < "$LOCAL_JOB"
"${SSH[@]}" "$KINDLE" "cmp /etc/upstart/$JOB -" < "$LOCAL_JOB"
echo "installed /etc/upstart/$JOB"
