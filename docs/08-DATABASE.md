# 08 - Database Design - EventStore + TigerBeetle + Read Models

## Two Databases, Not One

1. **Postgres EventStore** = Source of Truth for business events (immutable, append-only)
2. **TigerBeetle** = Source of Truth for money movements (double-entry balances)
3. **Postgres Read Models** = Materialized views for fast UI (rebuilt from 1+2)

## 1. EventStore (Postgres)

```sql
-- Enable pgcrypto
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Events - IMMUTABLE, never UPDATE or DELETE
CREATE TABLE events (
  event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  aggregate_type TEXT NOT NULL CHECK (aggregate_type IN ('Employee','Sale','Product','PayrollRun','Tenant')),
  aggregate_id UUID NOT NULL,
  version BIGINT NOT NULL,
  event_type TEXT NOT NULL,
  payload JSONB NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb, -- {user_id, terminal_id, ip, tenant_id}
  tenant_id TEXT NOT NULL, -- RNC for isolation
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  prev_hash TEXT,
  hash TEXT NOT NULL, -- SHA256(prev_hash || payload || version) - app calculates
  CONSTRAINT unique_version UNIQUE (aggregate_id, version)
);

CREATE INDEX idx_events_aggregate ON events (aggregate_id, version);
CREATE INDEX idx_events_tenant_type ON events (tenant_id, aggregate_type);
CREATE INDEX idx_events_type ON events (event_type);
CREATE INDEX idx_events_created ON events (created_at);

-- Snapshots for performance
CREATE TABLE snapshots (
  aggregate_id UUID PRIMARY KEY,
  aggregate_type TEXT NOT NULL,
  version BIGINT NOT NULL,
  state JSONB NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Event example:
-- INSERT INTO events (aggregate_type, aggregate_id, version, event_type, payload, tenant_id, prev_hash, hash)
-- VALUES ('Employee', 'uuid-emp', 1, 'EmployeeCreated', '{"name":"Juan","salary":20000}', '13079375', '0', 'abc...')
```

**Hash chaining:** App does `hash = SHA256(prev_hash + payload::text + version)`. On read, verify chain. Detects tampering (Formance pattern).

## 2. TigerBeetle Schema (Outside Postgres)

TigerBeetle has fixed schema: accounts + transfers, not tables.

**Account Model (per docs):**
```rust
struct Account {
  id: u128, // random UUID
  ledger: u32, // 700 = DOP
  code: u16, // chart code: 1=caja, 5=anticipos, etc.
  flags: AccountFlags,
  debits_pending: u128,
  debits_posted: u128,
  credits_pending: u128,
  credits_posted: u128,
  user_data_128: u128, // store tenant_id hash or employee_id
  user_data_64: u64,
  user_data_32: u32,
}
```

**Transfer Model:**
```rust
struct Transfer {
  id: u128, // idempotent - same ID = only processed once
  debit_account_id: u128,
  credit_account_id: u128,
  amount: u128, // in cents: 3000.00 DOP = 300000
  ledger: u32,
  code: u16, // 10=venta, 30=adelanto, 31=nomina
  flags: TransferFlags, // pending, post_pending, void_pending, linked
  user_data_128: u128, // sale_id or payroll_id
}
```

**Example Accounts Setup per Tenant (on TenantCreated event):**
```rust
// Tenant RNC 130793752 creates its chart
create_accounts([
  {id: tb_id("tenant:130793752:asset:caja"), code: 1, ledger: 700, flags: {}},
  {id: tb_id("tenant:130793752:asset:banco"), code: 2, ledger: 700, flags: {}},
  {id: tb_id("tenant:130793752:asset:anticipos"), code: 5, ledger: 700, flags: {}},
  {id: tb_id("tenant:130793752:liability:itbis"), code: 6, ledger: 700, flags: {}},
  ...
])
```

Deterministic ID: `tb_id = hash(tenant_id + account_name) as u128` so same tenant always same IDs.

## 3. Read Models (Postgres, Prisma)

These are built by projectors listening to events. They CAN be deleted/rebuilt.

```prisma
// packages/read-models schema.prisma or apps/web/prisma/schema.prisma

model EmployeeBalance {
  id            String   @id // employee_id
  tenantId      String   // RNC
  accruedNet    Decimal  @db.Decimal(12,2)
  advanceBalance Decimal @db.Decimal(12,2)
  availableAdvance Decimal @db.Decimal(12,2)
  lastClockIn   DateTime?
  updatedAt     DateTime @updatedAt
  @@index([tenantId])
}

model SaleRead {
  id            String @id // sale aggregate_id
  tenantId      String
  eNCF          String? @unique // E310000000001
  tipoECF       Int // 31,32
  total         Decimal
  itbisTotal    Decimal
  statusDGII    String // PENDING, ACEPTADO, RECHAZADO
  trackId       String?
  qrUrl         String?
  clientRnc     String?
  createdAt     DateTime
  @@index([tenantId, createdAt])
}

model AccountBalanceRead {
  id            String @id // tb account id as string
  tenantId      String
  accountName   String // asset:caja
  code          Int
  debitsPosted  Decimal
  creditsPosted Decimal
  balance       Decimal // computed: debits - credits or credits - debits depending on type
  updatedAt     DateTime
  @@index([tenantId])
}

model InventoryStockRead {
  sku           String @id
  tenantId      String
  name          String
  qtyOnHand     Int
  qtyReserved   Int
  qtyAvailable  Int // onHand - reserved
  avgCost       Decimal
  @@index([tenantId])
}

model PayrollRunRead {
  id            String @id
  tenantId      String
  period        String // 202607
  grossTotal    Decimal
  netTotal      Decimal
  deductionsTotal Decimal // includes advances
  status        String
  createdAt     DateTime
}
```

**Projector Logic (Rust):**
```rust
// On event SaleCompleted
async fn project_sale(event: Event) {
  let sale = parse::<SaleCompleted>(event.payload);
  sqlx::query("INSERT INTO SaleRead ...").execute(&pool).await;
  // Also update AccountBalanceRead from TigerBeetle lookup
  let tb_balances = tigerbeetle.lookup_accounts([...]).await;
  update_read_balances(tb_balances);
}
```

**Rebuild:** `DELETE FROM SaleRead; SELECT * FROM events WHERE event_type LIKE 'Sale%' ORDER BY created_at; replay`

## 4. DGII ECF Storage (S3)

Not Postgres. Signed XMLs stored in S3 compatible (Tigris / R2) for 10-year retention [DGII law].

```
s3://fiscal-xml-storage/
  tenant=130793752/
    year=2026/month=07/
      E310000000001.xml
      E310000000001.pdf (representation impresa)
      E310000000001_qr.png
```

Metadata in Postgres `ecf_documents` table with S3 URL pointer.

## 5. Data Flows Summary

```
Write Path:
  Next.js --Command--> Rust Core --validate--> EventStore (append) --TB transfer--> TigerBeetle --emit event--> Projector --update--> Read Model (Prisma)

Read Path:
  Next.js --query--> Prisma Read Model (fast) --display

Money Path (critical):
  Rust Core --linked transfers--> TigerBeetle (atomic, idempotent)

DGII Path:
  Event SaleCompleted --consumer--> Sign XML --DGII API--> Event ETicketAccepted
```

## Why Not Just Postgres for Money?

- Race conditions: Two cashiers sell last item, both check stock=1, both sell -> -1 stock. TigerBeetle `debits_must_not_exceed_credits` flag prevents at DB level.
- Double-entry invariant: Postgres app code must ensure debits=credits. TigerBeetle enforces at storage layer, impossible to create money out of thin air.
- Idempotency: Network retry sends same transfer ID -> TigerBeetle processes once, not double charge.

## Migrations

- EventStore: never delete columns, only add new event versions. Old events still replay.
- Read Models: normal Prisma migrations, can drop and rebuild from EventStore anytime.
- TigerBeetle: add new accounts with `create_accounts`, never delete.

