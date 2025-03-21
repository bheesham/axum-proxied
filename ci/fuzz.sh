#!/usr/bin/env bash
set -e

cargo fuzz run proxy_parse -- -max_total_time=30
cargo fuzz run extract_forwarded_interface -- -max_total_time=30
