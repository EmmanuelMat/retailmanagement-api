/**
 * Next.js -> Rust Core Client
 * Real DGII flow: build XML per Informe Tecnico v1.0 + XAdES-BES sign + seed auth + send + poll TrackID
 * UI en español, comunicación en inglés
 */

const CORE_HTTP = process.env.CORE_HTTP_URL || process.env.NEXT_PUBLIC_CORE_URL || "http://localhost:3001";

export interface SignRequest {
  tenantId: string;
  eNCF: string;
  tipoECF: number;
  xmlContent: string;
  p12Base64?: string;
  p12Password?: string;
  isContingency?: boolean;
}

export interface SimplePos {
  tenantRnc: string;
  razonSocial: string;
  direccion: string;
  eNCF: string;
  tipoECF: number;
  clienteRnc: string;
  clienteNombre: string;
  items: { nombre: string; cantidad: string; precio: string }[];
  fechaEmision: string; // DD-MM-YYYY
  fechaVencimiento: string;
}

export interface ECFJson {
  Encabezado: {
    Version: string;
    IdDoc: {
      TipoeCF: number;
      eNCF: string;
      FechaVencimientoSecuencia: string;
      IndicadorEnvioDiferido: number;
      TipoIngresos: string;
      TipoPago: number;
      TotalPaginas: number;
    };
    Emisor: {
      RNCEmisor: string;
      RazonSocialEmisor: string;
      DireccionEmisor: string;
      FechaEmision: string;
    };
    Comprador: {
      RNCComprador: string;
      RazonSocialComprador: string;
    };
    Totales: {
      MontoGravadoTotal?: number;
      MontoExento?: number;
      ITBIS1?: number;
      TotalITBIS?: number;
      MontoTotal: number;
    };
  };
  DetallesItems: {
    Item: Array<{
      NumeroLinea: number;
      IndicadorFacturacion: number;
      NombreItem: string;
      IndicadorBienoServicio: number;
      CantidadItem: number;
      PrecioUnitarioItem: number;
      MontoItem: number;
    }>;
  };
}

export async function signECFWithCore(req: SignRequest) {
  const res = await fetch(`${CORE_HTTP}/v1/ecf/sign`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      tenantId: req.tenantId,
      eNCF: req.eNCF,
      tipoECF: req.tipoECF,
      xmlContent: req.xmlContent,
      p12Base64: req.p12Base64,
      p12Password: req.p12Password,
      isContingency: req.isContingency || false,
    }),
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`Core signing failed ${res.status}: ${await res.text()}`);
  return res.json();
}

export async function signDemo() {
  const res = await fetch(`${CORE_HTTP}/v1/test/sign-demo`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({}),
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`Demo sign failed: ${await res.text()}`);
  return res.json();
}

export async function buildECF(simplePos?: SimplePos, ecf?: ECFJson) {
  const res = await fetch(`${CORE_HTTP}/v1/ecf/build`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ simplePos, ecf }),
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`Build ECF failed: ${await res.text()}`);
  return res.json() as Promise<{ xml: string; xml_preview: string; e_ncf: string; tipo_ecf: number }>;
}

export async function buildAndSignECF(params: { simplePos?: SimplePos; ecf?: ECFJson; p12Base64: string; p12Password?: string }) {
  const res = await fetch(`${CORE_HTTP}/v1/ecf/build-sign`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(params),
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`Build+Sign failed: ${await res.text()}`);
  return res.json() as Promise<{
    e_ncf: string;
    tipo_ecf: number;
    xml_built: string;
    signed_xml: string;
    codigo_seguridad: string;
    digest_value: string;
    qr_url: string;
    file_name: string;
  }>;
}

export async function buildSignAndSendToDGII(params: {
  simplePos?: SimplePos;
  ecf?: ECFJson;
  p12Base64: string;
  p12Password?: string;
  environment?: "TesteCF" | "CerteCF" | "eCF";
}) {
  const res = await fetch(`${CORE_HTTP}/v1/ecf/build-sign-send`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(params),
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`Build+Sign+Send DGII failed: ${await res.text()}`);
  return res.json() as Promise<{
    e_ncf: string;
    file_name: string;
    track_id: string;
    estado: string; // Aceptado, Rechazado
    codigo: number;
    codigo_seguridad: string;
    qr_url: string;
    dgii_mensajes: any;
    signed_xml_preview: string;
  }>;
}

export async function authenticateDGII(params: { p12Base64: string; p12Password?: string; environment?: string }) {
  const res = await fetch(`${CORE_HTTP}/v1/ecf/authenticate`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(params),
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`DGII auth failed: ${await res.text()}`);
  return res.json() as Promise<{ token: string; environment: string }>;
}

export async function getEmployeeBalance(tenantId: string, employeeId: string) {
  const res = await fetch(`${CORE_HTTP}/v1/employees/${employeeId}/balance?tenantId=${tenantId}`, { cache: "no-store" });
  if (!res.ok) throw new Error("Balance fetch failed");
  return res.json();
}

export async function requestAdvance(tenantId: string, employeeId: string, amount: string, reason: string) {
  const res = await fetch(`${CORE_HTTP}/v1/advances/request`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ tenantId, employeeId, amount, reason }),
  });
  if (!res.ok) throw new Error(`Advance failed: ${await res.text()}`);
  return res.json();
}
