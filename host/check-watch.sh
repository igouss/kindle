#!/usr/bin/env bash
set -uo pipefail

readonly bin="${1:-$HOME/.cargo/bin/kindle-watch}"
work="$(mktemp -d)"
readonly work
readonly log="$work/watch.log"
readonly old_schema="$HOME/Projects/beads_rust/sample_beads_db_files/repro_beadsrust_import_write.M6eaGY"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/proj" && cd "$work/proj" && git init -q && br init >/dev/null 2>&1
id=$(br create "acceptance probe" --description "## What

Pushed by the watcher." --json 2>/dev/null | jq -r '.id // .[0].id')
if [[ -d $old_schema/.beads ]]; then
    mkdir -p "$work/old" && cp -r "$old_schema/.beads" "$work/old/"
fi
"$bin" --root "$work" > "$log" 2>&1 & pid=$!
pushes(){ grep -c '^pushed' "$log"; }
queries(){ grep -c '^queried' "$log"; }
sleep 3; a=$(pushes); first=$(grep '^pushed' "$log" | tail -1)
br update "$id" --status in_progress >/dev/null 2>&1; sleep 3; b=$(pushes); second=$(grep '^pushed' "$log" | tail -1)
R="--no-auto-flush --no-auto-import --allow-stale"
q0=$(queries)
br list --status in_progress --json $R >/dev/null; br show "$id" --json $R >/dev/null; br audit log "$id" --json $R >/dev/null; sleep 3; c=$(pushes); q1=$(queries)
br close "$id" >/dev/null 2>&1; sleep 3; d=$(pushes); fourth=$(grep '^pushed' "$log" | tail -1)
kill $pid; wait $pid 2>/dev/null
echo "1 start:   pushes=$a  $first"
echo "2 claim:   pushes=$b  $second"
echo "3 reads:   pushes=$c  (expect $b), queries $q0 -> $q1 (expect no change)"
echo "4 close:   pushes=$d  $fourth"
old=$(grep -c "^queried $work/old:" "$log")
if [[ -d $work/old ]]; then
    echo "5 old schema: queried $old times (expect 1; its reads write a temp file)"
else
    echo "5 old schema: SKIP, no $old_schema"
fi
ok=1
[[ $a == 1 && $first == "pushed nothing:"* ]] || ok=0
[[ $b == 2 && $second == "pushed $id:"* ]] || ok=0
[[ $c == "$b" && $q0 == "$q1" ]] || ok=0
[[ $d == 3 && $fourth == "pushed nothing:"* ]] || ok=0
[[ ! -d $work/old || $old == 1 ]] || ok=0
grep -v '^pushed\|^watching\|^queried' "$log" | head -5
if [[ $ok == 1 ]]; then echo PASS; else echo FAIL; exit 1; fi
