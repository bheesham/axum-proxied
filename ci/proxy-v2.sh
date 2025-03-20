#!/usr/bin/env bash
set -e

cargo build --example proxy
cargo run --example proxy & SERVER_PID=$!
haproxy -f dev/haproxy.conf & HAPROXY_PID=$!

echo "running proxy example ($SERVER_PID)"
echo "  haproxy ($HAPROXY_PID)"
sleep 3

set +e
sh dev/test-proxy-v2.sh
STATUS=$?
set -e

kill -9 "$HAPROXY_PID"
kill -9 "$SERVER_PID"
exit $STATUS
