#!/usr/bin/env bash
set -e

curl -H 'Forwarded: for="_mdn"' "http://127.0.0.1:3000"
curl -H 'Forwarded: For="[2001:db8:cafe::17]:4711"' "http://127.0.0.1:3000"
curl -H "Forwarded: for=192.0.2.60;proto=http;by=203.0.113.43" "http://127.0.0.1:3000"
curl -H "Forwarded: for=192.0.2.43, for=198.51.100.17" "http://127.0.0.1:3000"
