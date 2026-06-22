#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Checking Docker daemon..."
if ! docker info >/dev/null 2>&1; then
  echo "ERROR: Docker daemon is not running. Start Docker Desktop and try again."
  exit 1
fi

echo "==> Starting Postgres container..."
docker-compose up -d

echo "==> Waiting for Postgres to be ready..."
for i in $(seq 1 30); do
  if docker-compose exec -T postgres pg_isready -U chatuser -d chatapp >/dev/null 2>&1; then
    echo "Postgres is ready."
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "ERROR: Postgres did not become ready in time."
    exit 1
  fi
  sleep 1
done

echo "==> Verifying schema..."
expected_tables="messages sessions users"
actual_tables=$(docker-compose exec -T postgres psql -U chatuser -d chatapp -tAc \
  "SELECT table_name FROM information_schema.tables WHERE table_schema='public' ORDER BY table_name;" \
  | tr -d '\r' | xargs)

if [ "$actual_tables" = "$expected_tables" ]; then
  echo "Schema OK: found tables -> $actual_tables"
else
  echo "ERROR: expected tables [$expected_tables] but found [$actual_tables]"
  exit 1
fi

echo "==> All checks passed."
