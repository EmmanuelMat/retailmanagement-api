# 07 - Monorepo Structure & Conventions

## Structure

```
root/
├── apps/
│   ├── web/                 # Next.js 15 - Modern POS UI + BFF API
│   │   ├── app/
│   │   │   ├── (pos)/page.tsx          # POS terminal
│   │   │   ├── (dashboard)/layout.tsx  # Accounting, inventory, payroll
│   │   │   ├── api/                    # BFF routes calling Rust core
│   │   │   │   ├── sales/route.ts
│   │   │   │   ├── advances/route.ts
│   │   │   │   └── reports/606/route.ts
│   │   │   ├── layout.tsx
│   │   │   └── globals.css
│   │   ├── components/ui/              # shadcn local copy
│   │   ├── lib/
│   │   │   ├── grpc-client.ts
│   │   │   ├── prisma.ts
│   │   │   └── utils.ts
│   │   ├── prisma/schema.prisma        # READ MODELS ONLY (not event store)
│   │   ├── package.json
│   │   └── tailwind.config.ts
│   └── mobile/              # Expo React Native
│       ├── src/
│       │   ├── screens/POS.tsx
│       │   ├── screens/Advances.tsx
│       │   └── lib/grpc-client.ts
│       ├── app.json
│       └── package.json
├── services/
│   └── core/                # Rust bank-core
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs                # Axum + Tonic servers
│       │   ├── lib.rs
│       │   ├── event_store/
│       │   │   ├── mod.rs
│       │   │   ├── postgres.rs
│       │   │   └── events.rs
│       │   ├── ledger/
│       │   │   ├── tigerbeetle.rs
│       │   │   └── chart_of_accounts.rs
│       │   ├── aggregates/
│       │   │   ├── employee.rs
│       │   │   ├── sale.rs
│       │   │   └── inventory.rs
│       │   ├── services/
│       │   │   ├── ecfl_service.rs    # DGII signing
│       │   │   ├── payroll_service.rs
│       │   │   └── report_service.rs
│       │   ├── proto/
│       │   │   └── fiscal.proto
│       │   └── bin/
│       │       └── migrate.rs
│       ├── docker-compose.yml
│       ├── Dockerfile
│       └── .env.example
├── packages/
│   ├── ui/                  # Shared shadcn UI
│   │   ├── src/button.tsx
│   │   ├── src/card.tsx
│   │   └── package.json
│   ├── api-client/          # TS client for Rust core
│   │   ├── src/index.ts
│   │   └── package.json
│   └── proto/               # Proto files + generated TS
│       ├── fiscal.proto
│       ├── payroll.proto
│       └── package.json
├── docs/                    # Documentation First
│   ├── 01-ARCHITECTURE.md
│   ├── 02-DGII-COMPLIANCE.md
│   ├── 03-BANK-CORE-EVENT-DRIVEN.md
│   ├── 04-RUST-CORE-RPC.md
│   ├── 05-PAYROLL-ADVANCE.md
│   ├── 06-API-SPEC.md
│   ├── 07-MONOREPO.md
│   ├── 08-DATABASE.md
│   └── README.md
├── package.json
├── pnpm-workspace.yaml
├── turbo.json
└── README.md
```

## Conventions

### 1. Documentation First
- No code without doc update in `/docs`
- Every PR must update relevant doc

### 2. No Float for Money
- Rust: `rust_decimal::Decimal`
- TS: store cents as string or `int`, display with `Intl.NumberFormat('es-DO')`

### 3. Event Naming
- Past tense: `SaleCompleted`, `AdvanceApproved`, not `CompleteSale`
- Commands: imperative: `RequestAdvance`, `CompleteSale`

### 4. TigerBeetle IDs
- Use Uuid v4 as u128: `Uuid::new_v4().as_u128()`
- Idempotent: same ID = same transfer (no double charge)

### 5. Multi-Tenancy
- Every gRPC call includes `tenant_id` (RNC)
- Every Postgres read model row has `tenant_id` index
- TigerBeetle: use `code` field for tenant grouping or separate ledgers (700 + tenant_offset)

### 6. Testing
- Rust: unit tests for aggregates (given events -> when command -> expect events)
- TS: Vitest for UI
- Integration: docker-compose up, run `cargo test --tests`

### 7. Env

`.env.example` in services/core:
```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/fiscal_core
TIGERBEETLE_CLUSTER=3000
TIGERBEETLE_REPLICA_COUNT=1
CORE_GRPC_PORT=50051
CORE_HTTP_PORT=3001
JWT_SECRET=your-secret
DGII_ENV=CERT # DEV, CERT, PROD
S3_BUCKET=fiscal-xml-storage
```

## Scripts

```bash
pnpm dev          # all apps via turbo
pnpm dev:web      # Next.js web only
pnpm dev:core     # Rust core with cargo watch
pnpm build        # builds all
```

## Git

- `main` = production
- `develop` = staging
- Feature branches: `feat/payroll-advance`, `feat/ecf-signer`
- Commit conventional: `feat(core): add TigerBeetle two-phase for advance`
