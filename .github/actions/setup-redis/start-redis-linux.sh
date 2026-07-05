#!/usr/bin/env bash
set -euo pipefail

if ! redis-server --daemonize yes; then
  echo "redis-server failed to start"
  exit 1
fi

deadline=$((SECONDS + 30))
until redis-cli ping > /dev/null 2>&1; do
  if (( SECONDS >= deadline )); then
    echo "Redis did not become ready within 30 seconds"
    exit 1
  fi
  sleep 1
done

echo "Redis is ready"
