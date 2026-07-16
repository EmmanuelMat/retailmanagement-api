# Bank-Core Architecture in Rust - Event-Driven POS + Accounting + Payroll Advance
**For Dominican Republic DGII-compliant small business platform**

---

### 1. Your New Requirement = You Are Building a Bank, Not a POS

What you just described:
> Core in Rust = Event-driven database like bank core
> POS + Inventory + Full Accounting + Payroll with advances deducted from paycheck

This is not a CRUD app. This is **3 systems in 1**:
1. **Core Banking Ledger** (double-entry, immutable, bi-temporal)
2. **ERP** (inventory, POS, accounting GL tied to DGII)
3. **Earned Wage Access (EWA) / Adelantos** (legal + accounting + payroll deduction)

This is exactly how modern neobanks are built. Good news: there are open-source references that prove this works in Rust.

---

### 2. Systems Like Yours Found (Research)

#### A. Ledger / Accounting Databases (Your Foundation)

**1. TigerBeetle - Purpose-built accounting database [1](https://news.ycombinator.com/item?id=32497565)[2](https://docs.tigerbeetle.com/start/)**
- Built in Zig, but has Rust client. **1M TPS**, Jepsen-tested for safety [3](https://jepsen.io/analyses/tigerbeetle-0.16.11)
- Data model = ONLY `accounts` and `transfers`. Enforces double-entry at database level, not app level [4](https://docs.tigerbeetle.com/single-page/)
- **Perfect for you:** It has `debits_pending` / `credits_posted` + two-phase transfers out of the box [1]. That means: reserve money for an advance, post it later atomically. No race conditions.
- Used by Interledger Rafiki as its ledger [5](https://interledger.org/developers/blog/rafiki-tigerbeetle-integration/)
- Flags like `debits_must_not_exceed_credits` prevent negative balances at DB level [6](https://medium.com/@altuntasfatih42/building-a-double-entry-ledger-with-elixir-and-tigerbeetle-f0f9fcc37408)

**2. Formance Stack - Open-Source Programmable Ledger [7](https://www.formance.com/blog/engineering/double-entry-accounting-for-engineers-building-financial-products)[8](https://github.com/formancehq/stack)**
- MIT license, Go but with Numscript DSL for transaction modeling [9](https://docs.formance.com/modules/ledger)
- **Key features for you:** 
  - Immutable, hash-chained like blockchain - detects retroactive edits [10](https://www.formance.com/blog/product/programmable-wallets-architecture-holds-and-the-ledger-layer)
  - Bi-temporality: "What did balance look like Tuesday 3pm before 4pm correction?" [7]
  - Multi-ledger isolation per tenant (perfect for SaaS - one ledger per business)
  - Source/destination model prevents creating money out of thin air [11](https://docs.formance.com/modules/ledger/accounting-model/source-destination-accounting-model)
- Used by Shares.io - 2 engineers deployed in 2 months what would have taken 5 engineers 6 months [12](https://www.formance.com/blog/customers/shares)

**Tradeoff:**
- TigerBeetle = raw speed + safety, you build GL logic yourself
- Formance = higher-level, DSL, audit-ready, slower but full accounting primitives

**My recommendation for you:** **TigerBeetle as low-level ledger + Formance-like logic in Rust on top.** TigerBeetle handles the money movement, Postgres holds chart of accounts metadata.

#### B. Rust Event Sourcing Frameworks (Your App Pattern)

Your "event-driven database like bank" = Event Sourcing + CQRS.

Found in Rust ecosystem:
- **eventsourced (hseeberger)** - Core traits + NATS & Postgres implementations [13](https://github.com/hseeberger/eventsourced)
- **eventually-rs** - Most feature-rich, supports projections + event store subscriptions [14](https://github.com/get-eventually/eventually-rs)[15](https://www.reddit.com/r/rust/comments/j571fr/eventually_eventsourcing_in_rust/)
- **Thalo.rs** - ESDL schema language to define aggregates/events, with Postgres & Kafka integration [16](https://www.reddit.com/r/rust/comments/sbvpn1/thalors_event_sourcing_in_rust/)
- **Nautilus Trader** - Production-grade Rust-native trading engine with deterministic event-driven architecture [17](https://github.com/nautechsystems/nautilus_trader) - Closest to bank core, written in Rust+Python

Pattern they all use:
```
Command -> Aggregate.handle_command() -> Event -> EventStore (append-only) -> Projectors -> Read Models
```
State is **never stored**, only rebuilt from events.

#### C. POS + Accounting + Payroll All-in-One (Your Competitors)

Researched ERPs:

| System | Accounting | Payroll Advance | POS | Why they fail as bank-core |
|---|---|---|---|---|
| **ERPNext** | Full double-entry, multi-currency | Basic, no EWA, manual loan [18](https://frappe.io/erpnext/comparisons/erpnext-vs-odoo) | Web POS, offline ok | No event sourcing, balance checks in app code -> race conditions |
| **Odoo Enterprise** | 40+ apps, polished [19](https://www.certumsolutions.com/library/intro-to-odoo-erp-all-in-one-business-software) | Enterprise only, no built-in advance deduction | Industry-specific | Per-user per-app pricing kills small RD business [$300-600/mo for 10 users] [20](https://eloerp.net/blog/eloerp-vs-odoo-vs-erpnext/) |
| **EloERP** | Built-in flat pricing [20] | Built-in payroll | Native multi-location | Not open, not event-driven |

**None are bank-grade.** They use mutable tables. You cannot answer "what was inventory value at 2025-12-15 14:00 before restatement?"

#### D. Payroll Advance / Earned Wage Access (Your Adelantos Feature)

This is a US fintech trend you are bringing to DR - huge differentiator.

**How leaders do it (Netchex, DailyPay, Payactiv, Tapcheck):**
- **It's NOT a loan.** It's access to already-earned wages [21](https://netchex.com/blog/zero-cost-benefits-how-earned-wage-access-reduces-turnover/)[22](https://www.dailypay.com/resource-center/blog/paycheck-advance/)
- **Flow:** Real-time accrual from timekeeping -> employee requests 50% of accrued net -> provider funds it -> deduction file sent to payroll -> deducted next paycheck automatically, no manual reconciliation [23](https://www.tapcheck.com/earned-wage-access)
- **3 repayment models:** Deduction model (you deduct and remit), Payroll intercept, Settlement [24](https://tax.thomsonreuters.com/news/on-demand-pay-off-schedule-compliance-what-payroll-leaders-need-to-know-about-earned-wage-access/)
- **Accounting trick:** Employer has zero liability if using provider model - Payactiv purchases receivable from employee [25](https://payactiv.com/trust-center/compliance-handbook)
- **Integration:** Connects via API to ADP, Paychex etc, streams hours [26](https://apps.adp.com/en-US/apps/215292/payactiv-earned-wage-access-for-adp-workforce-now)

Your DR version: Employee can get RD$ 2,000 adelanto mid-month, deducted from quincena.

Compliance: In DR, you must handle TSS and ISR before advance calculation, and reflect as `anticipo de salario` account, not salary expense yet.

---

### 3. Proposed Architecture - Event-Driven Bank Core in Rust

#### High-Level Layers (Wallet-Ledger Separation Pattern from Formance [10])

```
Layer 4: Wallet Experience (Next.js) -> UI, POS buttons
Layer 3: Policy & Permissions (Rust) -> Spend limits, ITBIS rules, advance limits (50%)
Layer 2: Orchestration (Rust) -> Converts business events to ledger postings, DGII calls
Layer 1: Core Ledger (TigerBeetle + EventStore Postgres) -> Double-entry immutable truth
```

#### Event Store Design

**DO NOT use Kafka yet.** For SME in DR, use **Postgres as Event Store** with:

```sql
-- Append-only, immutable
TABLE events (
  event_id UUID PRIMARY KEY,
  aggregate_type TEXT, -- Employee, Sale, InventoryItem, PayrollRun
  aggregate_id UUID,
  event_version BIGINT, -- optimistic locking
  event_type TEXT, -- EmployeeClockedIn, AdvanceRequested, SaleCompleted, ETicketSigned
  payload JSONB,
  metadata JSONB, -- who, when, ip, terminal
  created_at TIMESTAMPTZ,
  hash TEXT, -- SHA256(prev_hash + payload) for tamper-evidence like Formance
  UNIQUE(aggregate_id, event_version)
)

TABLE snapshots -- every 100 events for fast rebuild
```

This gives you bi-temporal + audit. TigerBeetle stores balances, Postgres stores business context.

#### Aggregate Examples in Rust

```rust
// Using eventually-rs pattern
#[derive(Aggregate)]
struct EmployeeAggregate {
  id: EmployeeId,
  accrued_net: Decimal, // real-time earned but unpaid
  advance_balance: Decimal, // how much taken
  salary: Decimal,
}

enum EmployeeCommand {
  ClockIn { hours: Decimal },
  RequestAdvance { amount: Decimal, reason: String }, // up to 50% of accrued_net - advance_balance
  RunPayroll { period: Period },
}

enum EmployeeEvent {
  HoursAccrued { amount: Decimal, at: DateTime },
  AdvanceRequested { amount: Decimal, request_id: Uuid },
  AdvanceApproved { amount: Decimal, request_id: Uuid, transfer_id: u128 }, // TigerBeetle transfer ID
  PayrollExecuted { gross: Decimal, tss: Decimal, isr: Decimal, net: Decimal, deductions: Vec<Deduction> },
}
```

#### Accounting as Events (Dominican Chart of Accounts)

Every POS sale emits **4 accounting events atomically via linked transfers in TigerBeetle**:

```
Sale RD$ 1,180 (RD$ 1000 + ITBIS 180)
  Transfer 1: Debit  client:cuenta_por_cobrar 1180 | Credit revenue:ventas 1000
  Transfer 2: Debit  client:cuenta_por_cobrar 1180 | Credit liability:itbis_por_pagar 180  (for IT-1 declaration)
  Transfer 3: (inventory) Debit cogs:costo_venta 600 | Credit inventory:mercancia 600
  All 3 linked -> atomic, all or none [1]
```

**Payroll Advance Accounting:**

```
Day 12: Employee earned RD$10k, requests RD$3k advance (50% rule OK)
  Event: AdvanceApproved
  TigerBeetle: Debit asset:anticipos_empleados (employee_id) 3000 | Credit asset:caja/banco 3000 (pending -> posted)
  
Day 15 Payroll:
  Gross 20,000
  Deductions: TSS 590, ISR 1000, Advance 3000
  Net to pay: 15,410
  TigerBeetle linked transfers:
    Debit expense:sueldos 20000 | Credit liability:tss_por_pagar 590
    Debit expense:sueldos... | Credit liability:isr_empleados 1000
    Debit expense:sueldos... | Credit asset:anticipos_empleados 3000 (settles advance)
    Debit expense:sueldos... | Credit asset:banco 15410
```

This is 100% audit-ready for DGII ISR.

#### Payroll Advance Engine Rules (EWA style)

```rust
struct AdvancePolicy {
  max_percentage: Decimal = 0.50, // 50% of accrued net [21]
  max_per_period: Decimal = 10000, // RD$
  min_balance_after: Decimal = 2000,
  fee: Decimal = 0, // free for first version, or RD$25
}

fn can_request_advance(employee: &EmployeeAggregate, amount: Decimal) -> bool {
  let available = employee.accrued_net - employee.advance_balance;
  let requested_pct = amount / employee.accrued_net;
  requested_pct <= 0.50 && available >= amount && employee.suspended == false
}
```

Real-time accrual: On each `ClockIn` or `Sale` (if commission), publish event, projector updates `accrued_net`.

Deduction: Payroll run reads `advance_balance`, creates `Deduction { type: Anticipo, amount }`, sends to ledger.

#### POS + Inventory as Event Sourcing

```
Command: AddItemToCart { sku, qty }
Event: ItemAddedToCart { sku, qty, price_at_time, itbis_at_time }

Command: CompleteSale { tipo_eCF: E32, client: Option<RNC> }
Events:
  - SaleCompleted { eNCF, total, itbis }
  - InventoryReserved { sku, qty } -> pending debit in TigerBeetle
  - ETicketSigningRequested { json_payload }
  - ETicketSigned { trackId, qr_url } -- async consumer calls DGII
  - InventoryCommitted { reserved -> posted }
```

If DGII rejects, emit `SaleVoided` compensating event + `InventoryReleased`. No manual fix.

#### DGII Integration as Event Consumer (Not blocking POS)

This is critical: **POS must NOT wait for DGII**. POS emits `SaleCompleted`, returns receipt with "En proceso DGII" + local QR. Background consumer (Rust tokio worker) consumes event bus, signs, sends to DGII, emits `ETicketAccepted` later. This is how banks handle Visa - authorize locally, settle async.

For offline colmado: Events stored in local SQLite queue (Tauri), synced when online.

---

### 4. Full Tech Stack Recommendation

**Core (Rust):**
- Runtime: Tokio + Axum (HTTP) + Tonic (gRPC)
- Event Store: Postgres 16 + sqlx + `eventually-rs` or `eventsourced`
- Ledger: **TigerBeetle** cluster (3 replicas for HA) - *you was right, Zig/Rust core is perfect*
- Decimal: `rust_decimal` (NEVER float for money)
- Crypto: `openssl`, `sha2`, `quick-xml` for DGII signing
- Message Bus for projections: NATS JetStream or Postgres LISTEN/NOTIFY for v1 (simpler)
- Snapshotting: Every 100 events per aggregate

**Edge (POS Terminal):**
- **Tauri 2.0** - Rust backend + Next.js frontend in WebView
- Local DB: SQLite (sqlx) + TigerBeetle single replica embedded? Or just queue to central TB when online
- 15MB binary, boots <1s on Windows POS

**Control Plane (Next.js):**
- Next.js 15 - server actions call Rust gRPC via `nice-grpc`
- Prisma only for read models (projections), NOT source of truth
- Dashboard reads from materialized views (e.g., `sales_daily`, `account_balances`)

**Infra for DR:**
- Fly.io in `scl` or `mia` (lowest latency to DGII)
- TigerBeetle cluster in same region
- S3 compatible (Tigris) for 10-year XML storage (DGII law)

---

### 5. Why This Architecture Wins in DR Market

| Feature | Traditional Odoo/ERPNext | Your Bank-Core Rust |
|---|---|---|
| **Audit** | Mutable rows, "who changed price?" lost | Full event log + hash chain, reconstruct any point in time [7] |
| **Concurrency** | Race: 2 cashiers sell last item -> negative stock | TigerBeetle `debits_must_not_exceed_credits` rejects atomically at DB [6] |
| **Payroll Advance** | Manual loan table, breaks ISR calc | Event-sourced accrued vs advance, atomic deduction [23] |
| **DGII 606/607** | Generated from mutable tables (wrong if edited) | Generated from immutable ledger, always balances |
| **Offline** | Caches, then conflicts | Event queue + idempotent transfers (TigerBeetle transaction IDs) [2] |

---

### 6. Roadmap to Build This

**Phase 1 - Ledger Foundation (2 weeks):**
- Setup TigerBeetle cluster + Rust wrapper
- Define Chart of Accounts for DR (Activos, Pasivos, Patrimonio, Ingresos, Costos, Gastos)
- Implement Numscript-like DSL in Rust: `send { source: ... dest: ... }`

**Phase 2 - Event Store + Employee Aggregate (3 weeks):**
- Postgres event table + aggregate trait
- EmployeeClocked -> Accrued, AdvanceRequest -> Check 50% rule -> TigerBeetle transfer
- Projector for `employee_balances` read model

**Phase 3 - POS & Accounting (4 weeks):**
- Sale aggregate + linked transfers for ITBIS
- DGII consumer async

**Phase 4 - Payroll Run (2 weeks):**
- Monthly payroll command consumes all employee aggregates, creates linked transfers for TSS, ISR, advances
- Generates Journal Entry for accounting + TXT for TSS (not DGII, but TSS is similar)

**Potential Risk:** Building TigerBeetle integration + XAdES signing from scratch is heavy. Start with Formance Ledger Docker for quick validation, migrate to TigerBeetle for performance.

Want me to scaffold the Rust core repo now? I can generate:
- `cargo new fiscal-core-bank`
- EventStore Postgres migrations
- TigerBeetle client setup
- Employee aggregate with Advance policy + tests
- Proto for gRPC to Next.js

This will be your base to pitch to investors too - "We are building TigerBeetle for Dominican SMEs".
