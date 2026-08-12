#!/usr/bin/env bash
# Single source of truth for the money e2e suite's environment. Sourced (not
# executed) by both scripts/e2e/run-local.sh and .github/workflows/e2e.yml so
# local runs and CI can never drift apart.
#
# JWT_SECRET / VENDOR_ADMIN_SECRET / LICENSE_SECRET / CERT_ENCRYPTION_KEY are
# required for services/core to start at all (main.rs panics without them)
# but are NOT real secrets here - services/core/.env.example documents them
# as `openssl rand -base64 32` with no fallback, and this Postgres/server pair
# is thrown away at the end of every run, so generating them fresh each time
# is safe and avoids checking any secret-shaped value into the repo.

export DATABASE_URL="postgres://postgres:postgres@localhost:5433/fiscal_core_test"
export JWT_SECRET="$(openssl rand -base64 32)"
export VENDOR_ADMIN_SECRET="$(openssl rand -base64 32)"
export LICENSE_SECRET="$(openssl rand -base64 32)"
export CERT_ENCRYPTION_KEY="$(openssl rand -base64 32)"
export CORE_HTTP_PORT="3001"
export CORE_HTTP_URL="http://localhost:3001"
export NEXT_PUBLIC_CORE_URL="http://localhost:3001"
export FRONTEND_URL="http://localhost:3000"
export RUST_LOG="fiscal_core=info"
# Isolate uploaded product images from a real dev checkout of services/core.
export UPLOADS_DIR="$(mktemp -d)"
