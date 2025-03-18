#!/usr/bin/env bash
set -e

# From https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Forwarded
curl -q -H 'Forwarded: for="_mdn"' "http://127.0.0.1:3000"
curl -q -H 'Forwarded: For="[2001:db8:cafe::17]:4711"' "http://127.0.0.1:3000"
curl -q -H "Forwarded: for=192.0.2.60;proto=http;by=203.0.113.43" "http://127.0.0.1:3000"
curl -q -H "Forwarded: for=192.0.2.43, for=198.51.100.17" "http://127.0.0.1:3000"

# From https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/X-Forwarded-For
curl -q -H "X-Forwarded-For: 2001:db8:85a3:8d3:1319:8a2e:370:7348" "http://127.0.0.1:3000"
curl -q -H "X-Forwarded-For: 203.0.113.195" "http://127.0.0.1:3000"
curl -q -H "X-Forwarded-For: 203.0.113.195, 2001:db8:85a3:8d3:1319:8a2e:370:7348" "http://127.0.0.1:3000"
