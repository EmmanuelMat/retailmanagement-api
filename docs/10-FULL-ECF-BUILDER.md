# 10 - Full ECF Builder per Informe Tecnico v1.0 + Real DGII Send

## What Was Added

### 1. ECF XML Builder (`services/core/src/ecf_builder.rs`)

Implements full DGII XSD per Informe Tecnico v1.0 and mseller docs JSON structure.

**Supports:**
- E31 Factura Crédito Fiscal Electrónica
- E32 Factura Consumo (<250k and >=250k)
- E33 Nota Débito, E34 Nota Crédito (via InformacionReferencia)
- E41 Compras, E43 Gastos Menores, E44 Regímenes Especiales, E45 Gubernamental, E46 Export, E47 Pagos Exterior
- All sections: Encabezado (Version, IdDoc, Emisor, Comprador, InformacionesAdicionales, Transporte, Totales), DetallesItems (Item[]), Paginacion, InformacionReferencia, Subtotales, DescuentosORecargos, FechaHoraFirma

**Core function:**

```rust
pub fn build_ecf_xml(ecf: &ECF) -> String
// Takes ECF struct -> returns XML string matching DGII XSD

pub fn build_simple_pos_ecf(
  tenant_rnc, razon_social, direccion,
  e_ncf, tipo_ecf,
  cliente_rnc, cliente_nombre,
  items: Vec<(nombre, qty, precio)>,
  fecha_emision DD-MM-YYYY, fecha_vencimiento
) -> ECF
// Convenience for colmado POS: qty*price = MontoItem, auto calc ITBIS 18%, MontoTotal
```

**Example JSON (mseller style) -> XML:**

Input JSON (from docs.ecf.mseller.app):

```json
{
  "ECF": {
    "Encabezado": {
      "Version": "1.0",
      "IdDoc": {
        "TipoeCF": 31,
        "eNCF": "E310000000001",
        "FechaVencimientoSecuencia": "31-12-2026",
        "IndicadorEnvioDiferido": 0,
        "TipoIngresos": "01",
        "TipoPago": 1,
        "TotalPaginas": 1
      },
      "Emisor": {
        "RNCEmisor": "130793752",
        "RazonSocialEmisor": "COLMADO EL SOL SRL",
        "DireccionEmisor": "Av Duarte",
        "FechaEmision": "15-07-2026"
      },
      "Comprador": {
        "RNCComprador": "131880657",
        "RazonSocialComprador": "CLIENTE SRL"
      },
      "Totales": {
        "MontoGravadoTotal": 1000.00,
        "MontoGravadoI1": 1000.00,
        "ITBIS1": 18,
        "TotalITBIS": 180.00,
        "MontoTotal": 1180.00
      }
    },
    "DetallesItems": {
      "Item": [{
        "NumeroLinea": 1,
        "IndicadorFacturacion": 1,
        "NombreItem": "Arroz Premium",
        "IndicadorBienoServicio": 1,
        "CantidadItem": 1,
        "PrecioUnitarioItem": 1000.00,
        "MontoItem": 1000.00
      }]
    }
  }
}
```

Output XML:

```xml
<?xml version="1.0" encoding="utf-8"?><ECF>
  <Encabezado>
    <Version>1.0</Version>
    <IdDoc><TipoeCF>31</TipoeCF><eNCF>E310000000001</eNCF><FechaVencimientoSecuencia>31-12-2026</FechaVencimientoSecuencia><IndicadorEnvioDiferido>0</IndicadorEnvioDiferido><TipoIngresos>01</TipoIngresos><TipoPago>1</TipoPago><TotalPaginas>1</TotalPaginas></IdDoc>
    <Emisor><RNCEmisor>130793752</RNCEmisor><RazonSocialEmisor>COLMADO EL SOL SRL</RazonSocialEmisor><DireccionEmisor>Av Duarte</DireccionEmisor><FechaEmision>15-07-2026</FechaEmision></Emisor>
    <Comprador><RNCComprador>131880657</RNCComprador><RazonSocialComprador>CLIENTE SRL</RazonSocialComprador></Comprador>
    <Totales><MontoGravadoTotal>1000.00</MontoGravadoTotal><MontoGravadoI1>1000.00</MontoGravadoI1><ITBIS1>18.00</ITBIS1><TotalITBIS>180.00</TotalITBIS><MontoTotal>1180.00</MontoTotal></Totales>
  </Encabezado>
  <DetallesItems><Item><NumeroLinea>1</NumeroLinea><IndicadorFacturacion>1</IndicadorFacturacion><NombreItem>Arroz Premium</NombreItem>...</Item></DetallesItems>
  <FechaHoraFirma></FechaHoraFirma>
</ECF>
```

Then this XML goes to `sign_xml_ecf()` for XAdES-BES signing.

### 2. Real DGII Client (`services/core/src/dgii_client.rs`)

Implements exact flow from `victors1681/dgii-ecf` RestApi.ts:

**Endpoints (per ENVIRONMENT):**

```rust
pub enum ENVIRONMENT { TesteCF, CerteCF, eCF }
BaseUrl ECF = https://ecf.dgii.gov.do
BaseUrl CF = https://fc.dgii.gov.do

ENDPOINTS:
  SEED = Autenticacion/api/Autenticacion/Semilla (GET)
  VALIDATE_SEED = autenticacion/api/Autenticacion/ValidarSemilla (POST multipart signed seed XML)
  SEND_INVOICE = recepcion/api/FacturasElectronicas (POST multipart signed eCF XML, fileName RNC+eNCF.xml)
  SEND_SUMMARY = recepcionfc/api/recepcion/ecf (for RFCE E32 <250k, POST to fc.dgii.gov.do)
  TRACK_RESULT_STATUS = consultaresultado/api/Consultas/Estado?trackId=...
```

**Flow methods:**

```rust
pub struct DGIIClient { environment, http_client, token }

impl DGIIClient {
  async fn get_seed(&self) -> String // GET semilla XML
  
  async fn authenticate_with_seed(&self, signed_seed_xml) -> AuthToken // POST multipart, returns { token }

  async fn authenticate(&mut self, p12_der, password) -> String // Full: get_seed -> sign_xml_ecf(seed, "SemillaModel") -> authenticate_with_seed -> token stored

  async fn send_ecf(&self, signed_ecf_xml, file_name) -> InvoiceResponse { trackId }

  async fn send_rfce(&self, signed_rfce_xml, file_name) -> InvoiceResponse

  async fn status_track_id(&self, track_id) -> TrackingStatusResponse { codigo, estado: Aceptado/Rechazado/AceptadoCondicional/EnProceso, mensajes }

  async fn send_with_polling(&self, signed_ecf_xml, file_name) -> TrackingStatusResponse
  // Does send_ecf -> loop poll status_track_id with exponential backoff 1s,2s,4s,8s up to 10 retries until final state
}
```

**Auth details:**

- Seed XML example: `<SemillaModel><Semilla>base64Random</Semilla><Codigo>...</Codigo></SemillaModel>`
- Sign seed same as e-CF but root "SemillaModel" - reuse sign_xml_ecf()
- POST to ValidarSemilla as multipart/form-data field "xml" file "seed.xml" with signed seed
- Response JSON: `{ "token": "eyJhbGciOiJIUzI1NiIs...", "expira": "..." }`
- Token used as `Authorization: Bearer <token>` for all subsequent calls, valid ~2 min

**Send details from dgii-ecf code:**

- Must send as multipart/form-data with knownLength, Content-Length calculated dynamically (DGII server rejects if not)
- Field name "xml", file path fileName = RNC + eNCF + ".xml" e.g., "130793752E320000000001.xml"
- For summary RFCE: same but to fc.dgii.gov.do endpoint

**Polling:**

```rust
let mut delay = 1s;
for attempt in 0..10 {
  sleep(delay);
  let status = status_track_id(trackId);
  match status.estado {
    Aceptado | AceptadoCondicional | Rechazado => return status,
    EnProceso | Recibido => { delay = min(delay*2, 8s); continue; }
  }
}
```

### 3. New HTTP Endpoints in Rust Core

Added in `main.rs`:

- `POST /v1/ecf/build` - Build XML only (no sign)
  Input: `{ ecf: {...} }` or `{ simplePos: { tenantRnc, razonSocial, direccion, eNCF, tipoECF, clienteRnc, clienteNombre, items: [{nombre, cantidad, precio}], fechaEmision, fechaVencimiento } }`
  Output: `{ xml, xml_preview, e_ncf, tipo_ecf }`

- `POST /v1/ecf/build-sign` - Build + Sign XAdES
  Input: same + `p12Base64` + `p12Password`
  Output: `{ xml_built, signed_xml, codigo_seguridad, digest_value, qr_url, file_name }`

- `POST /v1/ecf/build-sign-send` - Full flow: Build + Sign + Auth DGII (seed) + Send + Poll
  Input: same + environment: "TesteCF"/"CerteCF"/"eCF"
  Output: `{ e_ncf, file_name, track_id, estado: Aceptado/Rechazado, codigo, codigo_seguridad, qr_url, dgii_mensajes }`

- `POST /v1/ecf/authenticate` - Auth only
  Input: `{ p12Base64, p12Password, environment }`
  Output: `{ token, environment, message }`

- `GET /v1/ecf/status/:track_id?token=xxx&environment=TesteCF`
  Output: TrackingStatusResponse with estado

### 4. Example Full Flow via curl

```bash
# 1. Build only (no cert needed)
curl -X POST http://localhost:3001/v1/ecf/build \
  -H "Content-Type: application/json" \
  -d '{
    "simplePos": {
      "tenantRnc": "130793752",
      "razonSocial": "COLMADO EL SOL SRL",
      "direccion": "Av Duarte",
      "eNCF": "E320000000001",
      "tipoECF": 32,
      "clienteRnc": "000000000",
      "clienteNombre": "CONSUMIDOR FINAL",
      "items": [{"nombre": "Arroz Premium", "cantidad": "1", "precio": "1000"}],
      "fechaEmision": "15-07-2026",
      "fechaVencimiento": "31-12-2026"
    }
  }' | jq

# 2. Build + Sign with demo cert (self-signed P12 from /v1/test/sign-demo endpoint, but here we use gen)
# First get demo P12 base64 via endpoint that generates it internally - for real you need to pass your own P12 base64

# 3. Full flow with real P12 (you have INDOTEL cert for TesteCF)
# Suppose cert.p12 base64 encoded:
P12B64=$(base64 -w0 cert.p12)

curl -X POST http://localhost:3001/v1/ecf/build-sign-send \
  -H "Content-Type: application/json" \
  -d "{
    \"simplePos\": {
      \"tenantRnc\": \"130793752\",
      \"razonSocial\": \"COLMADO EL SOL SRL\",
      \"direccion\": \"Av Duarte\",
      \"eNCF\": \"E320000000001\",
      \"tipoECF\": 32,
      \"clienteRnc\": \"000000000\",
      \"clienteNombre\": \"CONSUMIDOR FINAL\",
      \"items\": [{\"nombre\": \"Arroz\", \"cantidad\": \"1\", \"precio\": \"1000\"}],
      \"fechaEmision\": \"15-07-2026\",
      \"fechaVencimiento\": \"31-12-2026\"
    },
    \"p12Base64\": \"$P12B64\",
    \"p12Password\": \"yourPassword\",
    \"environment\": \"TesteCF\"
  }" | jq

# Expected response when DGII accepts:
# {
#   "e_ncf": "E320000000001",
#   "file_name": "130793752E320000000001.xml",
#   "track_id": "d2b6e27c-3908-46f3-afaa-2207b9501b4b",
#   "estado": "Aceptado",
#   "codigo": 1,
#   "codigo_seguridad": "A1B2C3",
#   "qr_url": "https://ecf.dgii.gov.do/eCF/ConsultaTimbre?RNCEmisor=130793752&eNCF=E320000000001&...&CodigoSeguridad=A1B2C3",
#   "dgii_mensajes": [{"valor": "", "codigo": 0}]
# }
```

### 5. Next.js Wiring

Next.js now can call new endpoints via `apps/web/lib/core-client.ts`:

```ts
// Build only
await fetch(`${CORE_HTTP}/v1/ecf/build`, { method: "POST", body: JSON.stringify({ simplePos }) })

// Build + Sign
await fetch(`${CORE_HTTP}/v1/ecf/build-sign`, { method: "POST", body: JSON.stringify({ simplePos, p12Base64, p12Password }) })

// Full flow to DGII
await fetch(`${CORE_HTTP}/v1/ecf/build-sign-send`, { method: "POST", body: JSON.stringify({ simplePos, p12Base64, p12Password, environment: "TesteCF" }) })
```

`apps/web/app/api/sales/route.ts` updated to optionally use full flow if `sendToDGII=true` in body.

### 6. What Still Needs Real P12 for TesteCF?

To test against real DGII TesteCF, you need:

1. RNC registered as Emisor Electronico in DGII TesteCF environment (via OFV TesteCF)
2. P12 cert issued for that RNC for TesteCF (from DGII test cert provider, not production INDOTEL)
3. Sequence authorized: E32 range in TesteCF OFV

Then `build-sign-send` will actually hit https://ecf.dgii.gov.do/TesteCF/... and return real TrackID and estado Aceptado.

Without real cert, you can still test full builder + signer via `/v1/ecf/build-sign` using self-signed P12 from `/v1/test/sign-demo` logic.

Next: Implement RFCE XML builder for E32 <250k summary and ARECF/ACECF for reception.
