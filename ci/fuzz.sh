#!/usr/bin/env bash
set -e

cargo fuzz run proxy_parse -- -verbosity=0 -max_total_time=30
cargo fuzz run extract_forwarded_interface -- -verbosity=0 -max_total_time=30
