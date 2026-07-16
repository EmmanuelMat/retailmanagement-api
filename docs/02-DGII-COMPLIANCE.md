# DGII Compliance Investigation - Next.js Fullstack POS & Inventory for Dominican Republic
**Date:** July 2026 | **Location:** Santo Domingo, DO
**Objective:** Build a sellable Small Business POS + Inventory that is 100% compliant with DGII

---

### 1. Executive Summary
To sell a POS in the Dominican Republic in 2026, you cannot just print receipts. You MUST be a **Facturación Electrónica (e-CF)** system.

**Key Facts:**
*   **Law 32-23** makes electronic invoicing (e-CF) **mandatory for ALL taxpayers**. No more paper NCF will be legally valid after your deadline.
*   Deadline for **Small, Micro and Non-Classified** businesses (your target market) is **November 15, 2026** after a 6-month extension granted by DGII on May 6, 2026 [1](https://miscuentasrd.com/blog/que-es-el-ncf-republica-dominicana) [2](https://blog.alegra.com/republica-dominicana/obligatoriedad-de-factura-electronica/). Original was May 2026. Grandes and Medianos are already mandatory since Nov 15, 2025 [2](https://blog.alegra.com/republica-dominicana/obligatoriedad-de-factura-electronica/).
*   Your software must generate **XML e-CF** files, sign them with a **Digital Certificate (INDOTEL)**, send them to DGII in real-time, receive a **TrackID**, and generate a printable representation with **QR Code**.
*   It must also generate the monthly reports DGII requires: **606 (Purchases), 607 (Sales), 608 (Voided NCF), 609 (Foreign Payments)** before day 15 each month [3](https://siemprealdia.co/republica-dominicana/impuestos/herramienta-de-pre-validacion/).
*   Failing to comply = invoices rejected, client loses ITBIS credit, fines up to 50 minimum wages, and suspension of NCF authorization [2](https://blog.alegra.com/republica-dominicana/obligatoriedad-de-factura-electronica/).

### 2. Legal Framework
*   **Ley 32-23 de Facturación Electrónica** (May 2023) - creates e-CF system
*   **Decreto 587-24** - Regulation of Ley 32-23
*   **Norma General 06-2018** - Types and use of NCF
*   **Norma General 07-2018** - Obligation to send 606/607/608/609 formats
*   **Código Tributario Art. 338-345** - ITBIS rules [4](https://siemprealdia.co/republica-dominicana/impuestos/desglosando-los-aspectos-fundamentales-del-itbis/)

### 3. What DGII Compliance Means for a POS / Inventory

#### A) RNC - Registro Nacional de Contribuyentes
Every business client must have active RNC. Your app must:
- Validate RNC structure (9 or 11 digits for Cedula)
- Optionally consult DGII RNC status via scraping/API
- Store Client Supplier with Tipo Identificación: 1=RNC, 2=Cédula, 3=Passport

#### B) NCF vs e-CF - The Core
**NCF** (Número de Comprobante Fiscal) = 11 chars paper: B + type (01-17) + 8 digit sequential. e.g., B0100000522
**e-NCF / e-CF** = 13 chars electronic: E + type (31-47) + 10 digit sequential. e.g., E310000000005 [5](https://blog.alegra.com/republica-dominicana/numero-de-comprobante-fiscal/)

**You must stop thinking B01, B02. For new software, you must implement E31, E32 etc.** DGII is migrating completely.

**Mapping Table you MUST implement:**

| NCF Paper (old) | e-CF (NEW - your app) | Use | When your POS uses it |
|---|---|---|---|
| **B01** | **E31** | Factura Crédito Fiscal | Business client needs ITBIS credit. REQUIRES RNC. |
| **B02** | **E32** | Factura de Consumo | Consumer final. Most common in POS. No RNC needed if < RD$250,000 |
| **B03** | **E33** | Nota de Débito Electrónica | Add charge to previous invoice |
| **B04** | **E34** | Nota de Crédito Electrónica | Returns, discounts, void with value |
| **B11** | **E41** | Compras / Proveedor Informal | You buying from non-registered |
| **B13** | **E43** | Gastos Menores | Employee expenses |
| **B14** | **E44** | Regímenes Especiales | Zona Franca, CONFOTUR |
| **B15** | **E45** | Gubernamental | Sales to Government |
| **B16** | **E46** | Exportaciones | Export |
| **B17** | **E47** | Pagos al Exterior | Foreign supplier payments |

[1](https://miscuentasrd.com/blog/que-es-el-ncf-republica-dominicana) [5](https://blog.alegra.com/republica-dominicana/numero-de-comprobante-fiscal/)

**Rules:**
- Sequences are AUTHORIZED by DGII. You cannot invent them. Client requests range via Oficina Virtual DGII (OFV).
- Each sequence has FechaVencimiento.
- Never reuse, never skip if possible. Skips must be justified in audit.
- E32 < 250,000 DOP: DGII allows summary reporting via **Resumen Factura Consumo (RFCE)** - instead of sending each ticket, you send daily summary. CRITICAL for high-volume POS [6](https://ecf.mseller.app/). Your app MUST support this to not overload DGII.

#### C) ITBIS (Impuesto Transferencia Bienes Industrializados y Servicios)
Your product pricing and inventory must be ITBIS-aware:

*   **18% tasa general** - 90% of products/services [4](https://siemprealdia.co/republica-dominicana/impuestos/desglosando-los-aspectos-fundamentales-del-itbis/)
*   **16% tasa reducida** - yogur, café, azúcar, grasa comestible, cacao, chocolate [4](https://siemprealdia.co/republica-dominicana/impuestos/desglosando-los-aspectos-fundamentales-del-itbis/) [7](https://phlaw.com/the-itbis-in-the-dominican-republic-a-complete-guide-for-businesses-and-professionals/)
*   **0% / Exento** - basic foods (carne, leche, pan, frutas), educación, salud, transporte terrestre, alquiler vivienda, financieras [4](https://siemprealdia.co/republica-dominicana/impuestos/desglosando-los-aspectos-fundamentales-del-itbis/)

Each **Product in Inventory must have:** `itbisType: GRAVA_18 | GRAVA_16 | EXENTO | 0_EXPORT` and `indicadorFacturacion` per DGII spec.

Declaration: **Form IT-1** monthly before 20th.

#### D) Monthly Reports 606, 607, 608, 609

These are STILL mandatory even with e-CF, except some e-CF 100% emitters may be exempt from 607 complementary.

*   **606 - Compras:** All supplier invoices with NCF/e-NCF, RNC, date, montos facturados, ITBIS facturado, retenciones. Requires pre-validation before IT-1 [8](https://ayuda.dgii.gov.do/conversations/discusiones/llenado-de-formato-606/64f68fcc9445bf586cdcb90b)[9](https://blog.alegra.com/republica-dominicana/reportes-contables-606-607-608/)
*   **607 - Ventas:** All your issued NCF/e-NCF. Must detail clientes con RNC, ITBIS cobrado. If you issued E32 <250k, you must fill summary section "Resumen General de Facturas de Consumo" [9](https://blog.alegra.com/republica-dominicana/reportes-contables-606-607-608/)
*   **608 - Anulados:** List of NCF that were voided. Must specify motivo (DGII codes 1-10)
*   **609 - Pagos al Exterior**

Format: TXT/Excel template from DGII OFV, deadline 15th of next month. If no operations, send informative empty file [3](https://siemprealdia.co/republica-dominicana/impuestos/herramienta-de-pre-validacion/).

Your POS must auto-generate these with one click.

### 4. e-CF Technical Flow - What YOU Must Code

This is the official flow from DGII documentation [10](https://dgii.gov.do/cicloContribuyente/facturacion/comprobantesFiscalesElectronicosE-CF/Paginas/documentacionSobreE-CF.aspx):

**Prerequisites per client:**
1. RNC active, up-to-date in DGII
2. **Certificado Digital** issued by INDOTEL-authorized entity (Camara, Avansi, etc). Cost RD$3,000-7,000 / year [1](https://miscuentasrd.com/blog/que-es-el-ncf-republica-dominicana). Private key (.p12) stored securely server-side.
3. Authorized as Emisor Electrónico in OFV
4. Have sequential ranges authorized (E31, E32...)

**Emission Flow (per invoice):**
```
1. POS creates sale -> Your backend generates JSON -> Transform to XML per schema v1.0
   Structure: Encabezado (Version, IdDoc, Emisor, Comprador, Totales),
              DetallesItems (Item[]), Totals, Firma placeholder

2. Sign XML with XAdES-BES using private key (RSA-SHA256). 
   Generate Codigo Seguridad = first 6 digits of signature hash.

3. Authenticate to DGII: 
   POST /autenticacion with seed.xml signed -> receive token (valid 2-5 min)

4. Send: POST /recepcion with fileName = RNCEmisor+eNCF.xml + signed XML
   DGII returns TrackID immediately

5. DGII async validates: integrity, certificate valid, eNCF unique, RNC status, business rules.

6. Your system polls: GET /consultaResultado?trackId
   States: Aceptado, Rechazado, AceptadoCondicional

7. If Aceptado -> Generate Representación Impresa:
   PDF with: eNCF, RNC, QR code linking to https://ecf.dgii.gov.do/eCF/ConsultaTimbre?RNCEmisor=...&eNCF=...&RNCComprador=...&FechaEmision=...&MontoTotal=...&CodigoSeguridad=...
   Must be storable 10 YEARS per law [11](https://gosocket.net/todo-sobre-la-factura-electronica-republica-dominicana/)

8. Send to receptor: Email XML + PDF. If receptor is also electronic, send to their reception URL.
```

**For E32 < 250k (RFCE):**
- You generate individual E32 tickets locally (offline valid)
- At end of day you generate RESUMEN (RFCE) XML containing total of those E32s
- You send ONLY resumen to DGII. DGII guide says you save bandwidth.
- Implementation uses function convertECF32ToRFCE in open-source lib [12](https://github.com/victors1681/dgii-ecf)

**Contingencia (Offline Mode) - MANDATORY for POS:**
DGII Instructivo de Contingencia [10] says if internet fails, POS must continue facturando, store XML locally, mark `IndicadorEnvioDiferido=1` and send when connection returns with queue. Your Next.js app needs IndexedDB + Background Sync.

### 5. How to Make Your App Sellable - 2 Options

**OPTION A (Recommended to start): Become Integrated with Existing PSFE**
You don't become PSFE yourself. You use an API like:
- **eCF MSeller API** - Free tier 200 docs/month, has i npm `dgii-ecf` [12], community 2000 devs, handles signing, QR, DGII queue, 99.9% uptime [6]
- **Alanube**
- **Alegra API**

Your app sends JSON via REST, they sign and send to DGII, return QR. You pay per volume but save months of certification.

Your clients still need their own Digital Certificate, but the heavy XML logic is outsourced.

**OPTION B (Long-term if you want to be a Provider): Become Proveedor de Servicios de Facturación Electrónica Autorizado**
- You must certify YOUR software with DGII as PSFE [10]. List of authorized PSFE is on DGII portal.
- Process: desarrollas sistema propio, pasas set de pruebas de 25+ documentos de diferentes tipos en ambiente precertificación, demuestras seguridad, almacenamiento 10 años, alta disponibilidad.
- Time: 2-4 weeks for client, but for PSFE more.
- After certified, your clients select you as provider in OFV.

For MVP, start with Option A + using `dgii-ecf` library (open-source Node.js) [12] that already implements:
```
- Signature class
- Transformer json2xml
- ECF().authenticate(), sendElectronicDocument(), sendSummary(), getCustomerDirectory()
- getCodeSixDigitfromSignature()
```

### 6. Recommended Next.js Architecture

**Stack:**
- Next.js 15 App Router, TypeScript
- Prisma + PostgreSQL (multi-tenant via RLS or tenantId)
- NextAuth.js - roles: owner, cashier
- tRPC or Server Actions
- BullMQ + Redis for DGII async queue, retries, contingency
- S3/R2 for XML/PDF storage 10 years + encrypted certificates
- Tailwind + shadcn/ui for POS UI (fast)

**Core Tables:**
```prisma
Tenant { id, rnc, razonSocial, certificadoP12UrlEncrypted, passwordCertEncrypted, dgiiToken, ambientes: DEV/CERT/PROD }
Usuario
Producto { id, tenantId, nombre, sku, itbisTipo (16,18,0,EXENTO), costo, precio, indicadorBienoServicio (1=bien,2=serv), stock }
InventarioMovimiento
Cliente { id, rncOrCedula, tipoId, nombre, isProveedor }
Caja { id, abierta, montoApertura }
Venta { id, tenantId, eNCF (E310000000001), tipoeCF (31,32), secuencia, fechaEmision, fechaVencimientoSecuencia, montoGravadoTotal, itbisTotal, montoTotal, estadoDGII (PENDIENTE,ACEPTADO,RECHAZADO), trackId, codigoSeguridad, clienteId, esContingencia, rawXml, qrUrl }
VentaItem { ventaId, productoId, cantidad, precioUnit, itbis, montoItem }
ComprobanteAnulado { ncf, motivo, fecha }
 reporte606607 { periodo AAAAMM, tipo, contenidoTxt, estado }
```

**Modules:**

1. **Config DGII:** Upload P12, request sequences manually, register as emisor. Dashboard with expiring sequences.
2. **Inventario:** Kardex, multilocation, productos con ITBIS per tasa.
3. **POS:** 
   - Select tipo comprobante based on client: if client has RNC => suggest E31, if not => E32.
   - If E32 and total >250k => require RNC/Cedula (DGII rule).
   - Calculate ITBIS per item, handle propina legal (10%).
   - Offline mode: save to localStorage/IndexedDB, sync when online.
   - Print: ticket 80mm with QR, eNCF, RNC empresa, total, ITBIS desglosado.
4. **e-CF Engine Service:** Node microservice separate from Next.js edge. Handles signing (node-forge) and DGII comm. Never expose private key to client.
5. **Compras (606):** Register supplier invoices, validate eNCF via DGII consulta API, auto-calculate ITBIS credit.
6. **Reportes:** Generate 606,607,608 TXT with exact DGII layout, pre-validation runner, download.
7. **Dashboard Fiscal:** IT-1 draft, total ITBIS cobrado vs pagado, alerts before 15th.

### 7. Certification Steps for Your Clients (You automate this)
From DGII guide [10]:
1. Solicitud Emisor Electrónico en OFV (FI-GDF-016)
2. Submit appointment of admin user + representante
3. Firma de Postulación XML
4. Obtener Certificado Digital INDOTEL
5. Pruebas de precertificación: send at least 25 docs (E31,E32,E33,E34) to `https://ecf.dgii.gov.do` CERT environment
6. DGII approves => gives PROD credentials and sequences
7. Start facturando real

Your app should have wizard that guides them.

### 8. Checklist para Dev Compliance

- [ ] Implement all e-CF types E31-E47 in XML schema v1.0 (latest Informe Técnico updated 06/04/2026 [10])
- [ ] Sign XML with XAdES-BES, canonicalization C14N
- [ ] Generate QR per DGII spec (not custom)
- [ ] 10-year storage, immutable
- [ ] IndicadorEnvioDiferido for contingency
- [ ] RFCE summary for E32 <250k
- [ ] Validación RNC antes de emitir E31
- [ ] Secuencias con FechaVencimiento check before emit
- [ ] Nota Crédito E34 must reference original eNCF in InformacionReferencia
- [ ] 606/607/608 export in TXT exactly as DGII plantilla (23 columns for 606)
- [ ] ITBIS calculation with 18%/16%/exento split
- [ ] Representación Impresa models as DGII illustratives [10]
- [ ] Do not allow invoice without valid eNCF when mandatory

### 9. Next Steps Roadmap for You

**Week 1-2:** Setup Next.js boilerplate + multi-tenancy + Producto/Cliente/Venta models
**Week 3:** Integrate `dgii-ecf` npm locally, test signing with dummy cert
**Week 4:** Build POS UI offline-capable + printing + E32 RFCE logic
**Week 5:** Integrate MSeller or own DGII service + TrackID polling
**Week 6:** Generate 606/607/608 TXT + IT-1 preview
**Week 7:** Certification sandbox with DGII cert environment
**Week 8:** Pilot with one real small business (your own RNC de pruebas)

### Sources & Official Docs
- DGII e-CF Documentation Hub (Informe Técnico, Descripción Servicios, Instructivo Contingencia) [10]
- Calendario obligatoriedad e-CF [2][1]
- Tipos NCF [1][5]
- Formatos 606/607/608 [3][9]
- ITBIS [4]
- eCF MSeller format examples [13](https://docs.ecf.mseller.app/docs/integration/formato-documentos-ecf) [14](https://docs.ecf.mseller.app/docs/integration/document-format)
- Open source implementation [12]

> Estimate: Building PSFE-grade from scratch ~3-6 months. Building MVP integrated via third-party API ~4-6 weeks.

If you want, I can scaffold the Next.js project structure, Prisma schema and a working e-CF service example in your workspace next.
