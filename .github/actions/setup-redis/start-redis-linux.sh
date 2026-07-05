#!/usr/bin/env bash
set -euo pipefail

redis-server --daemonize yes || true
until redis-cli ping > /dev/null 2>&1; do sleep 1; done
