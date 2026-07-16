# 11 - RFCE + ARECF + ACECF - Resumen Consumo y Acuse Recibo

## RFCE - Resumen Factura Consumo (< RD$250k)

Per DGII Norma: E32 < RD$250,000 NO se envía individual a DGII, se envía resumen diario.

**Flow per dgii-ecf lib:**

```js
// 1. Firma E32 individual (guardas en tu DB)
const signedE32 = signature.signXml(xml32, 'ECF');
// 2. Extrae codigo seguridad = first 6 de hash SignatureValue
const { xml, securityCode } = convertECF32ToRFCE(signedE32);
// xml es RFCE entry con eNCF + MontoTotal + CodigoSeguridad
// 3. Firma RFCE
const signedRFCE = signature.signXml(rfceXml, 'RFCE');
// 4. Envía RFCE a https://fc.dgii.gov.do/{env}/recepcionfc/api/recepcion/ecf
// fileName = RNC + timestamp + .xml
// 5. Poll TrackID igual que E31
```

**Our Rust implementation `rfce_builder.rs`:**

```rust
pub struct RFCE {
  Encabezado { Version, IdDoc {FechaEmision, RNCEmisor}, Emisor {RNCEmisor, RazonSocialEmisor}, Totales {CantidadFacturas, MontoGravadoTotal, MontoExento, TotalITBIS, MontoTotal} },
  Detalles { FacturaConsumo: Vec<{eNCF, MontoTotal, CodigoSeguridad, FechaEmision}> },
  FechaHoraFirma
}

pub fn convert_ecf32_to_rfce_entry(signed_e32_xml) -> (FacturaConsumoResumen, codigoSeguridad)
  // Extract eNCF, MontoTotal via tag parsing
  // CodigoSeguridad = first 6 of SHA256(SignatureValue)

pub fn build_rfce_from_signed_e32_list(rnc, razon, fecha, Vec<signed_xml>) -> RFCE
  // Aggregates totals, counts

pub fn build_rfce_xml(rfce) -> String
```

**New endpoints:**

- `POST /v1/ecf/rfce/build` - Build RFCE XML from list of signed E32 XMLs
  Body: `{ rncEmisor, razonSocialEmisor, fechaEmision DD-MM-YYYY, signedE32XmlList: ["<ECF>...signed...</ECF>", ...] }`
  -> `{ xml, cantidadFacturas, montoTotal }`

- `POST /v1/ecf/rfce/build-sign` - Build + Sign RFCE with P12
  Body: same + `p12Base64` -> returns signed RFCE + file_name

- `POST /v1/ecf/rfce/build-sign-send` - Full: Build + Sign + Auth DGII + Send to fc.dgii.gov.do + Poll TrackID
  Body: same + environment TesteCF/CerteCF/eCF + P12
  -> `{ track_id, estado: Aceptado, cantidadFacturas, montoTotal }`

**Example colmado daily close:**

```bash
# Day with 47 E32 <250k, you have them signed and stored

curl -X POST http://localhost:3001/v1/ecf/rfce/build \
  -H "Content-Type: application/json" \
  -d '{
    "rncEmisor": "130793752",
    "razonSocialEmisor": "COLMADO EL SOL SRL",
    "fechaEmision": "15-07-2026",
    "signedE32XmlList": ["<ECF>...E320000000001...</ECF>", "<ECF>...E320000000002...</ECF>"]
  }'

# With real P12 and send to DGII TesteCF
curl -X POST http://localhost:3001/v1/ecf/rfce/build-sign-send \
  -H "Content-Type: application/json" \
  -d '{
    "rncEmisor": "130793752",
    "razonSocialEmisor": "COLMADO EL SOL SRL",
    "fechaEmision": "15-07-2026",
    "signedE32XmlList": [...],
    "p12Base64": "MIIJRA...",
    "p12Password": "pass",
    "environment": "TesteCF"
  }'
```

This is used by POS at day close (cierre de caja) to send daily summary.

---

## ARECF - Acuse de Recibo (Obligatorio automático)

Per DGII: Cuando eres Receptor Electrónico y recibes e-CF de un proveedor, debes enviar ARECF.

Structure:

```xml
<ARECF>
  <Encabezado>
    <Version>1.0</Version>
    <IdDoc><TipoeCF>31</TipoeCF><eNCF>E310000000001</eNCF><RNCEmisor>130793752</RNCEmisor><FechaEmision>15-07-2026</FechaEmision></IdDoc>
    <Emisor><RNCEmisor>131880657</RNCEmisor><RazonSocialEmisor>MI EMPRESA RECEPTORA</RazonSocialEmisor></Emisor>
    <Receptor><RNCReceptor>130793752</RNCReceptor><RazonSocialReceptor>COLMADO EL SOL</RazonSocialReceptor></Receptor>
  </Encabezado>
  <Detalles>
    <Estado>0</Estado> <!-- 0=Recibido, 1=No Recibido -->
    <FechaHoraRecepcion>15-07-2026 21:33:14</FechaHoraRecepcion>
  </Detalles>
  <FechaHoraFirma></FechaHoraFirma>
</ARECF>
```

**Builder `arecf_acecf_builder.rs`:**

```rust
pub fn build_arecf_recibido(tipoECF, eNCF, rncEmisorOriginal, fechaEmisionOriginal, rncReceptor, razonReceptor, rncEmisorRec, razonEmisorRec) -> ARECF
pub fn build_arecf_xml(arecf) -> String
```

**Endpoints:**

- `POST /v1/ecf/arecf/build` - Build ARECF XML (no sign)
  Body: `{ tipoECF:31, eNCF, rncEmisorOriginal, fechaEmisionOriginal, rncReceptor, razonSocialReceptor }`

- `POST /v1/ecf/arecf/build-sign` - Build + Sign with your P12 (you as receptor)
  Body: same + p12Base64
  -> signed ARECF ready to send to Emisor (fe/recepcion/api/ecf) and DGII (recepcion/api/FacturasElectronicas? Actually ARECF has its own endpoint? But spec says send to both)

---

## ACECF - Aprobación Comercial (Opcional)

Optional but recommended: Receptor indicates conformidad.

Estados:
- 0 = Aceptada
- 1 = Rechazo con reparos
- 2 = Rechazada

Structure:

```xml
<ACECF>
  <Encabezado>
    <Version>1.0</Version>
    <IdDoc><TipoeCF>31</TipoeCF><eNCF>E310000000001</eNCF><RNCEmisor>130793752</RNCEmisor></IdDoc>
    <Emisor><RNCEmisor>131880657</RNCEmisor><RazonSocialEmisor>RECEPTOR</RazonSocialEmisor></Emisor>
    <Receptor><RNCReceptor>130793752</RNCReceptor><RazonSocialReceptor>EMISOR ORIGINAL</RazonSocialReceptor></Receptor>
  </Encabezado>
  <Detalles>
    <Estado>0</Estado>
    <FechaHoraAprobacionComercial>15-07-2026 21:35:00</FechaHoraAprobacionComercial>
  </Detalles>
  <FechaHoraFirma></FechaHoraFirma>
</ACECF>
```

**Builder:**

```rust
pub fn build_acecf_aceptada(tipoECF, eNCF, rncEmisorOriginal, rncReceptor, razonReceptor, rncEmisorRec, razonEmisorRec) -> ACECF
pub fn build_acecf_xml(acecf) -> String
```

**Endpoints:**

- `POST /v1/ecf/acecf/build` - Build ACECF (0=Aceptada by default)
  Body: `{ tipoECF, eNCF, rncEmisorOriginal, rncReceptor, razonSocialReceptor, estado:0 }`

- `POST /v1/ecf/acecf/build-sign` - Build + Sign

Per Norma: If you don't send ACECF, e-CF is considered accepted when you report it in Form 606. But for automation, we send it.

---

## Full Flow POS with RFCE + ARECF/ACECF for Complete DGII Compliance

**Day in colmado with your POS:**

1. **Ventas E32 <250k** (47 ventas):
   - POS -> Event VentaCompletada -> build_simple_pos_ecf -> sign XAdES -> save signed XML + QR -> print ticket with QR
   - Do NOT send each to DGII, store in local DB + TigerBeetle
   - Event: ETicketSigningRequested, ETicketLocallyStored

2. **Cierre de caja 6pm**:
   - Gather all signed E32 XMLs <250k of day
   - Call `POST /v1/ecf/rfce/build-sign-send` with list + P12
   - Rust core: build_rfce_from_signed_e32_list -> sign -> auth DGII -> send to fc.dgii.gov.do -> poll TrackID
   - Event: RFCEEnviado { trackId, estado: Aceptado, cantidad:47 }
   - Print resumen cierre

3. **Recepción de proveedor** (you as receptor):
   - Proveedor envía E31 to your URL `fe/recepcion/api/ecf` (you need to expose this endpoint in Next.js that proxies to Rust)
   - Your system: build ARECF recibido -> sign -> send back to proveedor fe url + DGII
   - Optional: build ACECF aceptada (0) if you accept goods -> sign -> send to proveedor + DGII
   - Event: ProveedorECFRecibido, ARECFEnviado, ACECFAceptada
   - Accounting: auto-create 606 entry for that purchase, debit inventory, credit cuentas por pagar

This completes the full DGII cycle per Informe Tecnico: Emisor->DGII->Receptor->ARECF->ACECF->DGII.

All implemented in Rust core now.
