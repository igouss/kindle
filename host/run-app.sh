#!/usr/bin/env bash
set -euo pipefail

readonly KINDLE=kindle
readonly SSH=(ssh -o BatchMode=yes -o LogLevel=ERROR)
readonly SCP=(scp -q -o BatchMode=yes -o LogLevel=ERROR)
readonly TARGET=armv7-unknown-linux-musleabi
readonly REMOTE_DIR=/mnt/us/kindle-dev
readonly ROOT="$(dirname "$(readlink -f "$0")")/.."

app="${1:?usage: run-app.sh <crate> [args...]}"
shift

cargo build --quiet --release --manifest-path "$ROOT/Cargo.toml" --target "$TARGET" -p "$app"
"${SSH[@]}" "$KINDLE" "mkdir -p $REMOTE_DIR"
"${SCP[@]}" "$ROOT/target/$TARGET/release/$app" "$KINDLE:$REMOTE_DIR/$app"
"${SSH[@]}" "$KINDLE" "$REMOTE_DIR/$app" "$@"
