#!/usr/bin/env bash
set -euo pipefail

readonly KINDLE=kindle
readonly SSH=(ssh -o BatchMode=yes -o LogLevel=ERROR)
readonly HOST_IP=192.168.15.201
readonly PORT=8765
readonly INTERNET_URL="${INTERNET_URL:-https://example.com/}"
readonly REMOTE_SCRIPT=/mnt/us/kindle-dev/net-check.sh
readonly LOCAL_SCRIPT="$(dirname "$(readlink -f "$0")")/../kindle/net-check.sh"

serve_dir="$(mktemp -d)"
server_pid=""
cleanup() {
    [ -n "$server_pid" ] && kill "$server_pid"
    rm -rf "$serve_dir"
}
trap cleanup EXIT

nonce="$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n')"
printf '%s' "$nonce" > "$serve_dir/nonce"
python3 -m http.server "$PORT" --bind "$HOST_IP" --directory "$serve_dir" >/dev/null 2>&1 &
server_pid=$!
sleep 1

if "${SSH[@]}" "$KINDLE" true; then
    echo "PASS host_to_kindle"
else
    echo "FAIL host_to_kindle: ssh $KINDLE failed"
    exit 1
fi

"${SSH[@]}" "$KINDLE" "mkdir -p $(dirname "$REMOTE_SCRIPT") && cat > $REMOTE_SCRIPT" < "$LOCAL_SCRIPT"
"${SSH[@]}" "$KINDLE" "sh $REMOTE_SCRIPT http://$HOST_IP:$PORT/nonce $nonce '$INTERNET_URL'"
