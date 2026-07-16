# 01 - System Architecture - Bank-Grade Monorepo

## Vision: Build TigerBeetle for Dominican SMEs

We are not building CRUD POS. We are building:

> **Event-Driven Bank Core in Rust + Modern Next.js POS + Mobile + Full Accounting + Payroll Advances + DGII e-CF**

```
┌─────────────────────────────────────────────────────────┐
│ apps/web - Next.js 15 + Tailwind + shadcn/ui            │
│ - POS terminal (offline-capable)                        │
│ - Inventory                                             │
│ - Accounting Dashboard                                  │
│ - Payroll / Adelantos UI                                │
│ - API Routes -> gRPC client to Rust core                │
└──────────────────────┬──────────────────────────────────┘
                       │ gRPC (Tonic) / HTTP + MessagePack
┌──────────────────────▼──────────────────────────────────┐
│ services/core - fiscal-core Rust                        │
│ ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐ │
│ │ EventStore  │  │ TigerBeetle  │  │ DGII ECF Engine │ │
│ │ Postgres    │  │ Ledger DB    │  │ Signer + Queue  │ │
│ │ events      │  │ accounts     │  │ XAdES-BES       │ │
│ │ snapshots   │  │ transfers    │  │ TrackID poller  │ │
│ └─────────────┘  └──────────────┘  └─────────────────┘ │
│ ┌─────────────┐  ┌──────────────┐  ┌─────────────────┐ │
│ │ Aggregates  │  │ Projectors   │  │ Payroll Engine  │ │
│ │ Employee    │──▶│ materialized │  │ Accrual + 50%   │ │
│ │ Sale        │  │ views        │  │ Deduction       │ │
│ │ Inventory   │  │              │  │                 │ │
│ └─────────────┘  └──────────────┘  └─────────────────┘ │
└──────────────────────┬──────────────────────────────────┘
                       │ NATS / Postgres NOTIFY
┌──────────────────────▼──────────────────────────────────┐
│ DGII eCF API + TSS + S3 (10 year storage)               │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│ apps/mobile - Expo RN - same gRPC client, calls core    │
└─────────────────────────────────────────────────────────┘
```

## Event Sourcing Foundation

**Source of Truth:** `events` table (append-only, immutable, hash-chained)

```sql
CREATE TABLE events (
  event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  aggregate_type TEXT NOT NULL, -- 'Employee', 'Sale', 'Product', 'PayrollRun'
  aggregate_id UUID NOT NULL,
  version BIGINT NOT NULL,
  event_type TEXT NOT NULL, -- 'SaleCompleted', 'AdvanceApproved', etc.
  payload JSONB NOT NULL,
  metadata JSONB, -- user_id, terminal_id, ip
  created_at TIMESTAMPTZ DEFAULT NOW(),
  prev_hash TEXT,
  hash TEXT GENERATED ALWAYS AS (sha256(prev_hash || payload::text)) STORED,
  UNIQUE(aggregate_id, version)
);
CREATE INDEX ON events (aggregate_id, version);
CREATE INDEX ON events (event_type);
```

**Snapshot Table:** Every 100 events per aggregate for fast rebuild.

**Why:** You can answer "What was cash balance Tuesday 3pm before correction at 4pm?" - critical for DGII audits.

## Ledger Layer - TigerBeetle

TigerBeetle is embedded in Rust core as client, but runs as separate cluster (3 replicas for HA).

### Chart of Accounts for DR Small Business

```
Ledger 700 = DOP (Peso Dominicano) - per TigerBeetle docs ledger = asset type

Accounts (128-bit IDs):
1: asset:caja_efectivo (POS cash)
2: asset:banco_popular
3: asset:inventario_mercancia
4: asset:cuentas_por_cobrar (clientes)
5: asset:anticipos_empleados (adelantos)
6: liability:itbis_por_pagar (for IT-1)
7: liability:tss_por_pagar
8: liability:isr_retenido
9: liability:sueldos_por_pagar
10: revenue:ventas_gravadas_18
11: revenue:ventas_gravadas_16
12: revenue:ventas_exentas
13: expense:costo_venta
14: expense:sueldos
15: expense:alquiler
...
```

**Transfer Codes:**
- 10 = Venta POS
- 20 = Compra proveedor
- 30 = Adelanto empleado (pending -> posted two-phase [1])
- 31 = Pago nómina con deducción adelanto
- 40 = E-CF ITBIS posting

**Two-Phase for Adelantos:**
TigerBeetle supports `flags: pending` -> later `post_pending_transfer` or `void_pending_transfer`. This is perfect:
Advance requested = create pending transfer reserving funds, approve = post, reject = void.

## Aggregates

### 1. SaleAggregate
```
Commands: CreateSale, AddItem, ApplyDiscount, CompleteSale(with tipo e-CF)
Events: SaleCreated, ItemAdded, SaleCompleted, ETicketSigningRequested, ETicketAccepted, SaleVoided
```

### 2. InventoryAggregate
```
Commands: ReceiveStock, ReserveStock, CommitStock, ReleaseStock
Events: StockReceived, StockReserved (pending), StockCommitted, StockReleased
TigerBeetle integration: inventory:mercancia credits/debits
```

### 3. EmployeeAggregate (Core for advance)
```
Commands: ClockIn(hours), RequestAdvance(amount), ApproveAdvance, RunPayroll
Events: HoursAccrued, AdvanceRequested, AdvanceApproved(transfer_id), PayrollExecuted
State: accrued_net = sum(HoursAccrued) - sum(AdvanceApproved) - sum(PayrollExecuted)
Policy: can_request = amount <= (accrued_net * 0.50) && amount >= min
```

### 4. PayrollRunAggregate
```
Commands: StartPayrollRun(period)
Events: PayrollStarted, EmployeePayrollCalculated(gross, tss, isr, adelanto_deduction), PayrollCommitted
Projects to: accounting journal entries + TSS file + bank file
```

## CQRS - Read Models (Projectors)

EventStore is write-only. Projectors (Rust tokio tasks listening to Postgres NOTIFY) build read models in separate Postgres schemas for fast UI:

- `read_sales_daily` (for POS dashboard)
- `read_inventory_stock` (current stock)
- `read_employee_balances` (accrued, advance_balance)
- `read_account_balances` (mirror TigerBeetle but with metadata)
- `read_dgii_606_607` (pre-formatted for TXT export)

Next.js never queries EventStore directly. It queries read models via Prisma.

## DGII Integration as Async Consumer

```
POS -> Event SaleCompleted -> REST 200ms with local receipt "En proceso DGII"
       \
        -> NATS JetStream topic "sale.completed" -> Rust worker:
           1. Transform JSON -> XML (quick-xml)
           2. Sign XAdES-BES (openssl crate) - P12 decrypted from Vault
           3. Auth to DGII, POST /recepcion -> TrackID
           4. Loop poll /consultaResultado
           5. Emit event ETicketAccepted(trackId, qr_url) -> projector updates sale row
           6. If rejected, emit SaleVoided + notify POS via WebSocket
```

Offline: If DGII down / internet down, worker marks `IndicadorEnvioDiferido=1`, stores in local queue, retries.

## Payroll Advance Detailed Flow

See `05-PAYROLL-ADVANCE.md`

## Security

- P12 certificates encrypted with AES-256-GCM, key from env, decrypted only inside Rust enclave, zeroized after use (Rust `zeroize` crate)
- TigerBeetle transaction IDs = Uuid v4 for idempotency - prevents double advance
- Hash chain in events table detects tampering (like Formance)

## Deployment

- **Local Dev:** docker-compose.yml spins Postgres 16 + TigerBeetle single replica + NATS
- **Cloud:** Fly.io (mia region) -> 3 VM TigerBeetle cluster + Postgres (Neon/Supabase) + Rust core
- **Edge POS:** Tauri binary bundles Rust core + SQLite + TigerBeetle client in offline mode

## References

- TigerBeetle docs [2](https://docs.tigerbeetle.com/start/)
- Formance wallet-ledger separation [3](https://www.formance.com/blog/product/programmable-wallets-architecture-holds-and-the-ledger-layer)
