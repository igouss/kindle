#!/bin/sh
set -u

readonly HOST_URL="$1"
readonly NONCE="$2"
readonly INTERNET_URL="$3"
readonly KINDLE_IP=192.168.15.244

failures=0

pass() { echo "PASS $1"; }
fail() { echo "FAIL $1: $2"; failures=$((failures + 1)); }

if [ "$(lipc-get-prop com.lab126.cmd wirelessEnable)" = 0 ]; then
    pass wifi_off
else
    fail wifi_off "wireless is enabled; a successful fetch would not prove the USB path"
fi

if route -n | awk '$1 == "0.0.0.0" && $8 == "usb0" { found = 1 } END { exit !found }'; then
    pass default_route_usb0
else
    fail default_route_usb0 "no default route through usb0"
fi

body=$(curl -sS --max-time 10 "$HOST_URL" 2>&1)
if [ "$body" = "$NONCE" ]; then
    pass kindle_to_host
else
    fail kindle_to_host "expected nonce '$NONCE', got '$body'"
fi

result=$(curl -sS -o /dev/null --max-time 20 -w '%{local_ip} %{http_code}' "$INTERNET_URL" 2>&1)
if [ "$result" = "$KINDLE_IP 200" ]; then
    pass internet_via_usb
else
    fail internet_via_usb "expected '$KINDLE_IP 200', got '$result'"
fi

[ "$failures" -eq 0 ]
