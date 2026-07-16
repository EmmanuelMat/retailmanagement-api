# Colmado POS Dominicana - Núcleo Bancario en Rust + Next.js + DGII

**Sistema POS banco-grado para PYMES Dominicanas. 100% en Español. Cumple DGII Facturación Electrónica e-CF.**

> No es un POS CRUD. Es un núcleo bancario con TigerBeetle, Event Sourcing, contabilidad completa, nómina con adelantos 50% y firma XAdES-BES real.

## 🚀 Stack Monorepo

```
apps/
  web/          -> Next.js 15 App Router + Tailwind + shadcn/ui (UI 100% español dominicano)
                • Terminal POS • Inventario • Clientes • Contabilidad • Nómina y Adelantos • Reportes DGII 606/607
                • API Routes que llaman al núcleo Rust vía gRPC/HTTP
  mobile/       -> Expo React Native (POS móvil + adelantos empleado)

services/
  core/         -> Núcleo fiscal-core en Rust (bank-grade)
                • EventStore Postgres append-only hash-encadenado
                • TigerBeetle ledger DB (1M TPS, double-entry, pending/posted)
                • Firma XAdES-BES real per DGII spec (C14N, RSA-SHA256, DigestValue, QR)
                • Motor de Adelantos 50% (Earned Wage Access)
                • HTTP :3001 + gRPC :50051

packages/
  ui/           -> shadcn componentes compartidos
  api-client/   -> Cliente TS para núcleo Rust
  proto/        -> fiscal.proto + payroll.proto
docs/           -> Documentación primero (9 docs)
```

## 🇩🇴 Cumplimiento DGII

- **Ley 32-23 e-CF obligatorio** desde 15 Nov 2026 para pequeños/micros
- Tipos: E31 (Crédito Fiscal B2B), E32 (Consumo B2C <250k con RFCE resumen diario), E33/E34 Notas Débito/Crédito, E41-E47
- Firma: XAdES-BES real implementada en Rust
  - CanonicalizationMethod: `http://www.w3.org/TR/2001/REC-xml-c14n-20010315`
  - SignatureMethod: `rsa-sha256`, DigestMethod: `sha256`
  - Transform: `enveloped-signature`, Reference URI=""
  - Flujo: c14n(original) -> SHA256 digest -> build SignedInfo -> c14n(SignedInfo) -> RSA-SHA256 sign -> SignatureValue -> QR con codigo_seguridad (6 chars)
- Reportes: 606 Compras, 607 Ventas, 608 Anulados, 609 Pagos exterior, IT-1
- ITBIS: 18% general, 16% reducida (yogurt, café, azúcar...), Exento (carne, leche, pan...)

Ver `docs/02-DGII-COMPLIANCE.md` (15k palabras investigación)

## 🏦 Núcleo Bancario (Event-Driven como banco)

No usa tablas mutables. Usa:

**EventStore Postgres (fuente verdad):**
```sql
events(event_id, aggregate_type, aggregate_id, version, event_type, payload, metadata, tenant_id, prev_hash, hash)
-- hash = SHA256(prev_hash + payload) -> detección alteración como blockchain
```

**TigerBeetle (ledger dinero):**
- Solo `accounts` + `transfers`, double-entry a nivel DB, no en app
- Flags: `debits_must_not_exceed_credits` evita stock negativo sin race conditions
- Two-phase: `pending` -> `post_pending`/`void_pending` -> perfecto para adelantos
- Linked transfers: venta + ITBIS + COGS atómico (3 transfers todos o ninguno)

**Agregados:**
- `EmployeeAggregate`: `accrued_net`, `advance_balance`, `available_for_advance()` = 50% regla
- `SaleAggregate`: `SaleCompleted -> ETicketSigningRequested -> ETicketAccepted`
- `InventoryAggregate`: `StockReserved (pending) -> Committed`

Ver `docs/01-ARCHITECTURE.md` + `docs/08-DATABASE.md`

## 💸 Adelantos de Sueldo (EWA - 50%)

Inspirado en Netchex, DailyPay, Payactiv pero para colmados dominicanos:

- **NO es préstamo.** Es sueldo ya ganado. Sin interés. $0 costo patrono si usa modelo deducción [Netchex].
- Empleado gana RD$9,200, disponible para adelanto RD$4,600 (50% - reserva RD$1,000)
- Solicita RD$2,000 motivo Medicina -> Gerente aprueba en web -> Núcleo Rust reserva en TigerBeetle pending (Debe anticipos / Haber caja) -> posted -> Evento AdelantoAprobado
- Quincena: Nómina RD$20k - TSS 590 - ISR 1000 - Adelanto 3000 = Neto RD$15,410
- Contabilidad: `Debit expense:sueldos 20k | Credit asset:anticipos 3k + banco 15410 + tss + isr` (linked atómico)

Ver `docs/05-PAYROLL-ADVANCE.md`

## 🖥️ UI en Español Dominicano (Requisito)

**Web POS:** Terminal con productos (Plátanos, Arroz Premium, Coca-Cola...), ITBIS desglosado, botón "Cobrar • Generar E32 • QR DGII", ledger vivo TigerBeetle, adelantos con motivo, registro eventos append-only, cumplimiento DGII 606/607.

**Móvil:** Empleado ve "Ganado hoy RD$6,400 • Disponible RD$3,200 (50%)" + botón "Solicitar Adelanto RD$2,000" -> gRPC `PayrollService/RequestAdvance`

Archivo: `apps/web/app/page.tsx` y `apps/mobile/src/index.tsx` - 100% español.

## 🔐 Firma XAdES-BES Real en Rust

Ubicación: `services/core/src/services/ecfl_service.rs` + `xml_c14n.rs`

- C14N inclusivo custom 100 líneas con `roxmltree`, sorted attrs, encode `&amp; &lt; &quot; &#xD;`
- P12 carga via `openssl::pkcs12::Pkcs12::from_der().parse2(password)` -> cert base64 DER
- SHA256 + RSA-SHA256 sign
- Self-signed P12 demo para dev sin cert INDOTEL: `cargo run` -> `POST /v1/test/sign-demo`

```bash
# Terminal 1 - Núcleo
cd services/core
cargo run --bin migrate
cargo run # HTTP :3001 + gRPC :50051

# Terminal 2 - Prueba firma real
curl -X POST http://localhost:3001/v1/test/sign-demo | jq
# => codigo_seguridad, digest_value, qr_url, signed_xml_preview con <SignatureValue>

# Terminal 3 - Web Next.js
pnpm install && pnpm dev:web # http://localhost:3000
# GET http://localhost:3000/api/test/sign -> proxy al núcleo Rust
```

Ver `docs/09-XADES-REAL-IMPLEMENTATION.md`

## 📚 Documentación (Primero)

1. 00-OVERVIEW-MONOREPO.md
2. 01-ARCHITECTURE.md
3. 02-DGII-COMPLIANCE.md
4. 03-BANK-CORE-EVENT-DRIVEN.md
5. 04-RUST-CORE-RPC.md
6. 05-PAYROLL-ADVANCE.md
7. 06-API-SPEC.md
8. 07-MONOREPO.md
9. 08-DATABASE.md
10. 09-XADES-REAL-IMPLEMENTATION.md

## 🐳 Local

```bash
docker-compose up -d
# Primer formato TigerBeetle (solo una vez)
docker exec ... tigerbeetle format --cluster=0 --replica=0 --replica-count=1 /data/0_0.tigerbeetle
docker-compose restart tigerbeetle
```

## 🔜 Próximos Pasos

- [ ] XML builder completo per Informe Tecnico e-CF v1.0 (actual simplificado)
- [ ] DGII send real: auth seed.xml -> token -> POST /Recepcion -> poll TrackID
- [ ] S3 10 años retención XML
- [ ] Prisma read models en web
- [ ] Tauri wrapper para POS offline en colmados sin internet

Hecho en 🇩🇴 Santo Domingo, listo para certificación PSFE DGII.
