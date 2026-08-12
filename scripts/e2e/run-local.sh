#!/usr/bin/env bash
# Runs the full money e2e suite locally, the same way .github/workflows/e2e.yml
# runs it in CI: ephemeral Postgres -> migrate -> real backend -> backend e2e
# tests -> real Next.js app -> Cypress against both, driven together.
#
# Usage: pnpm test:e2e   (or: bash scripts/e2e/run-local.sh)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

CORE_PID=""
WEB_PID=""

cleanup() {
  echo "--- cleanup ---"
  [ -n "$CORE_PID" ] && kill "$CORE_PID" 2>/dev/null || true
  [ -n "$WEB_PID" ] && kill "$WEB_PID" 2>/dev/null || true
  # Belt-and-suspenders: `pnpm start` doesn't always forward SIGTERM to the
  # `next start` process it spawns, which would otherwise leave port 3000
  # held after this script exits. Whatever's actually bound to our ports
  # gets killed directly too.
  lsof -ti :3001 2>/dev/null | xargs kill 2>/dev/null || true
  lsof -ti :3000 2>/dev/null | xargs kill 2>/dev/null || true
  docker compose -f docker-compose.test.yml down -v || true
}
trap cleanup EXIT

wait_for() {
  local url="$1" name="$2" tries=0
  until curl -sf "$url" >/dev/null 2>&1; do
    tries=$((tries + 1))
    if [ "$tries" -ge 60 ]; then
      echo "Timed out waiting for $name at $url" >&2
      exit 1
    fi
    sleep 1
  done
  echo "$name is up ($url)"
}

echo "--- starting ephemeral postgres ---"
docker compose -f docker-compose.test.yml up -d --wait

echo "--- loading test env ---"
# shellcheck source=/dev/null
source scripts/e2e/env.sh

echo "--- running migrations ---"
(cd services/core && cargo run --bin migrate)

echo "--- building + starting fiscal-core ---"
(cd services/core && cargo build --bin fiscal-core)
(cd services/core && exec ./target/debug/fiscal-core) &
CORE_PID=$!
wait_for "http://localhost:3001/health" "fiscal-core"

echo "--- running backend money e2e tests ---"
# Scoped to just the new e2e test binaries (services/core/tests/*.rs), not a
# bare `cargo test`: this repo's lib also carries a handful of pre-existing
# `#[cfg(test)]` unit tests in unrelated DGII e-CF signing code
# (src/ecf_builder.rs) that currently fail to compile against its own
# updated function signature - a real, separate bug, but out of scope for
# this money-correctness suite (DGII e-CF is explicitly excluded from this
# suite's scope) and not something this suite should be blocked by.
(cd services/core && cargo test --no-fail-fast \
  --test ventas_ledger \
  --test compras_gastos_ledger \
  --test caja_bancos \
  --test nomina_adelantos \
  --test nomina_run \
  --test ledger_invariant)

echo "--- building + starting web app ---"
pnpm --filter web build
pnpm --filter web start &
WEB_PID=$!
wait_for "http://localhost:3000" "web"

echo "--- running Cypress e2e tests ---"
pnpm --filter web exec cypress run

echo "--- e2e suite passed ---"
