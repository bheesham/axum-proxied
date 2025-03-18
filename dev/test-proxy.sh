#!/usr/bin/env bash
set -e

curl -q --haproxy-clientip "0.0.0.0" "http://127.0.0.1:3000"
