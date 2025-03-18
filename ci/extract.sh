#!/usr/bin/env bash
set -e

cargo build --example extract
cargo run --example extract & SERVER_PID=$!
echo "running extract example ($SERVER_PID)"
sleep 3

set +e
sh dev/test-extract.sh
STATUS=$?
set -e

kill -9 "$SERVER_PID"
exit $STATUS
