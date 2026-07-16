# 00 - Monorepo Overview - What We Built

## Created Files

Run `tree -L 4 -I 'node_modules|target'` to see:

```
.
├── apps/
│   ├── web/ - Next.js 15 modern POS UI (bank-grade dashboard)
│   │   ├── app/page.tsx - Full POS + Ledger + Adelantos + Event Log UI
│   │   ├── app/api/sales/route.ts - BFF calling Rust core
│   │   └── app/api/advances/request/route.ts
│   └── mobile/ - Expo RN (same Rust core client)
├── services/
│   └── core/ - Rust fiscal-core
│       ├── src/main.rs - Axum HTTP + future Tonic gRPC
│       ├── src/event_store/mod.rs - Append-only events + hash chain + NOTIFY
│       ├── src/aggregates/employee.rs - 50% advance rule + tests
│       ├── src/aggregates/sale.rs - Sale + DGII flow
│       ├── src/aggregates/inventory.rs
│       ├── src/ledger/mod.rs - TigerBeetle client + pending/post for advances
│       ├── src/services/ecfl_service.rs - XAdES signing + QR
│       ├── src/services/payroll_service.rs - Advance request -> TB reserve -> approve
│       ├── src/services/report_service.rs - 606/607 TXT generator
│       ├── Cargo.toml - dependencies (tokio, axum, tonic, sqlx, rust_decimal, openssl, quick-xml)
│       └── docker-compose.yml
├── packages/
│   ├── ui/ - Shared shadcn button+card
│   ├── api-client/ - TS client for Rust core (HTTP fallback + future gRPC)
│   └── proto/ - fiscal.proto + payroll.proto (gRPC contracts)
├── docs/
│   ├── 01-ARCHITECTURE.md - System diagram + event store schema
│   ├── 02-DGII-COMPLIANCE.md - Copied from investigation
│   ├── 03-BANK-CORE-EVENT-DRIVEN.md - TigerBeetle vs Formance research
│   ├── 04-RUST-CORE-RPC.md - Rust RPC design
│   ├── 05-PAYROLL-ADVANCE.md - EWA model + accounting entries
│   ├── 06-API-SPEC.md - gRPC spec
│   ├── 07-MONOREPO.md - Conventions
│   └── 08-DATABASE.md - EventStore + TigerBeetle + Read models
├── package.json + pnpm-workspace.yaml + turbo.json
└── docker-compose.yml - Postgres + TigerBeetle + NATS for local dev
```

## How to Run (Documentation First Done)

```bash
# 1. Start infra
docker-compose up -d

# 2. First time format TigerBeetle (only once)
docker exec rd-pos-bank-core-monorepo-tigerbeetle-1 /tigerbeetle format --cluster=0 --replica=0 --replica-count=1 /data/0_0.tigerbeetle
docker-compose restart tigerbeetle

# 3. Run Rust core migrations
cd services/core
cargo run --bin migrate

# 4. Run Rust core
cargo run

# 5. In another terminal, run web
pnpm install
pnpm dev:web -> http://localhost:3000

# 6. Mobile
cd apps/mobile
pnpm start
```

## Core Principles Implemented

- **Event sourcing**: `event_store/mod.rs` appends with hash chain + pg_notify
- **TigerBeetle pending/post**: `ledger/mod.rs` reserve_advance for payroll advances
- **50% rule**: `aggregates/employee.rs` available_for_advance()
- **No float**: `rust_decimal` everywhere
- **DGII async**: POS returns fast, DGII consumer async (ETicketSigningRequested -> ETicketAccepted)

## Next Steps

- [ ] Implement real XAdES-BES signing in `ecfl_service.rs` (currently mock)
- [ ] Connect sqlx pool + real TigerBeetle client (currently mock)
- [ ] Add Prisma read models in apps/web
- [ ] Generate TS from proto with `buf` or `prost`
- [ ] Tauri wrapper for offline POS
