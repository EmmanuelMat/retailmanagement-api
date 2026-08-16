"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { ArrowLeft, Building2, LayoutGrid, Mail, ShieldCheck, Upload, Plus, Users, KeyRound } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label, Select, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@repo/ui";
import { apiFetch } from "@/lib/staff-api";

interface TenantResumen {
  rnc: string;
  razon_social: string;
  correo: string | null;
  telefono: string | null;
  license_status: "trial" | "active" | "expired";
  trial_started_at: string;
  trial_days: number;
  license_activated_at: string | null;
  activo: boolean;
  created_at: string;
}

interface ModuloAsignado {
  codigo: string;
  nombre: string;
  descripcion: string | null;
  activo: boolean;
}

interface Empresa {
  rnc: string;
  razon_social: string;
  nombre_comercial: string | null;
  direccion: string;
  telefono: string | null;
  correo: string | null;
  logo_url: string | null;
  ambiente_dgii: string;
  factura_electronica_activa: boolean;
}

interface CertificadoStatus {
  id: string;
  nombre_archivo: string;
  activo: boolean;
  uploaded_at: string;
}

interface SecuenciaNcf {
  id: string;
  tipo_ecf: number;
  prefijo: string;
  desde: number;
  hasta: number;
  proximo: number;
  fecha_vencimiento: string;
  estado: string;
}

interface UsuarioTenant {
  id: string;
  nombre: string;
  email: string;
  rol: string;
  rol_id: string | null;
  activo: boolean;
  created_at: string;
}

interface RolResumen {
  id: string;
  codigo: string;
  nombre: string;
  es_admin: boolean;
}

const TIPOS_ECF = [
  { value: 31, label: "31 · Crédito Fiscal" },
  { value: 32, label: "32 · Consumo" },
  { value: 33, label: "33 · Nota de Débito" },
  { value: 34, label: "34 · Nota de Crédito" },
  { value: 41, label: "41 · Compras" },
  { value: 43, label: "43 · Gastos Menores" },
];

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve((reader.result as string).split(",")[1] || "");
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

function diasRestantes(t: TenantResumen): number {
  const inicio = new Date(t.trial_started_at).getTime();
  const transcurridos = Math.floor((Date.now() - inicio) / 86400000);
  return Math.max(t.trial_days - transcurridos, 0);
}

const TABS = [
  { key: "modulos", label: "Módulos", icon: LayoutGrid },
  { key: "negocio", label: "Mi Negocio", icon: Building2 },
  { key: "dgii", label: "DGII", icon: ShieldCheck },
  { key: "usuarios", label: "Usuarios", icon: Users },
] as const;

export default function TenantDetailPage() {
  const params = useParams<{ rnc: string }>();
  const router = useRouter();
  const rnc = params.rnc;
  const [tab, setTab] = useState<(typeof TABS)[number]["key"]>("modulos");

  const [tenant, setTenant] = useState<TenantResumen | null>(null);
  const [error, setError] = useState("");
  const [activando, setActivando] = useState(false);
  const [reenviando, setReenviando] = useState(false);
  const [mensaje, setMensaje] = useState("");

  async function load() {
    try {
      const t = await apiFetch<TenantResumen>(`/api/tenants/${rnc}`);
      setTenant(t);
    } catch (e: any) {
      setError(e.message);
    }
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rnc]);

  async function activarLicencia() {
    setActivando(true);
    setError("");
    setMensaje("");
    try {
      await apiFetch(`/api/tenants/${rnc}/activar-licencia`, { method: "POST" });
      setMensaje("Licencia activada.");
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setActivando(false);
    }
  }

  async function reenviarInvitacion() {
    setReenviando(true);
    setError("");
    setMensaje("");
    try {
      await apiFetch(`/api/tenants/${rnc}/reenviar-invitacion`, { method: "POST" });
      setMensaje("Invitación reenviada.");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setReenviando(false);
    }
  }

  if (!tenant) {
    return (
      <div className="space-y-4">
        {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}
        {!error && <p className="text-sm text-muted-foreground">Cargando...</p>}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <Button variant="ghost" size="sm" onClick={() => router.push("/tenants" as any)} className="mb-2 -ml-2">
          <ArrowLeft className="h-4 w-4" /> Negocios
        </Button>
        <div className="flex items-start justify-between gap-4 flex-wrap">
          <div>
            <div className="flex items-center gap-2.5">
              <h1 className="text-2xl font-bold tracking-tight">{tenant.razon_social}</h1>
              {tenant.license_status === "active" ? (
                <Badge variant="success">Activo</Badge>
              ) : tenant.license_status === "expired" ? (
                <Badge variant="destructive">Vencido</Badge>
              ) : (
                <Badge variant={diasRestantes(tenant) <= 14 ? "warning" : "secondary"}>Prueba · {diasRestantes(tenant)}d</Badge>
              )}
            </div>
            <p className="text-sm text-muted-foreground mt-1 font-mono">{tenant.rnc} {tenant.correo ? `· ${tenant.correo}` : ""}</p>
          </div>
          <div className="flex gap-2">
            <Button variant="secondary" onClick={reenviarInvitacion} disabled={reenviando || !tenant.correo}>
              <Mail className="h-4 w-4" />{reenviando ? "Enviando..." : "Reenviar invitación"}
            </Button>
            {tenant.license_status !== "active" && (
              <Button onClick={activarLicencia} disabled={activando}>
                <ShieldCheck className="h-4 w-4" />{activando ? "Activando..." : "Activar licencia"}
              </Button>
            )}
          </div>
        </div>
      </div>

      {mensaje && <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">{mensaje}</div>}
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <div className="flex gap-1 border-b border-border">
        {TABS.map((t) => {
          const Icon = t.icon;
          return (
            <button
              key={t.key}
              onClick={() => setTab(t.key)}
              className={`flex items-center gap-1.5 px-3.5 py-2.5 text-sm font-medium border-b-2 -mb-px transition-colors ${
                tab === t.key ? "border-primary text-primary" : "border-transparent text-muted-foreground hover:text-foreground"
              }`}
            >
              <Icon className="h-4 w-4" />
              {t.label}
            </button>
          );
        })}
      </div>

      {tab === "modulos" && <ModulosTab rnc={rnc} />}
      {tab === "negocio" && <NegocioTab rnc={rnc} />}
      {tab === "dgii" && <DgiiTab rnc={rnc} />}
      {tab === "usuarios" && <UsuariosTab rnc={rnc} />}
    </div>
  );
}

function ModulosTab({ rnc }: { rnc: string }) {
  const [modulos, setModulos] = useState<ModuloAsignado[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    apiFetch<ModuloAsignado[]>(`/api/tenants/${rnc}/modulos`)
      .then(setModulos)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [rnc]);

  function toggle(codigo: string) {
    setModulos((prev) => prev.map((m) => (m.codigo === codigo ? { ...m, activo: !m.activo } : m)));
    setSaved(false);
  }

  async function guardar() {
    setSaving(true);
    setError("");
    try {
      const codigos = modulos.filter((m) => m.activo).map((m) => m.codigo);
      const data = await apiFetch<ModuloAsignado[]>(`/api/tenants/${rnc}/modulos`, {
        method: "PUT",
        body: JSON.stringify({ codigos }),
      });
      setModulos(data);
      setSaved(true);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  if (loading) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  return (
    <Card className="max-w-2xl">
      <CardContent className="pt-5 space-y-1">
        <p className="text-sm text-muted-foreground mb-4">
          Qué módulos ve este negocio una vez su licencia esté activa. Durante la prueba ven todo el sistema, sin
          importar lo marcado aquí — esto solo aplica al plan pagado.
        </p>
        {modulos.map((m) => (
          <label key={m.codigo} className="flex items-start gap-3 py-2.5 border-b border-border last:border-0 cursor-pointer">
            <input type="checkbox" checked={m.activo} onChange={() => toggle(m.codigo)} className="h-4 w-4 mt-0.5 rounded border-border" />
            <div className="min-w-0">
              <p className="text-sm font-medium">{m.nombre}</p>
              {m.descripcion && <p className="text-xs text-muted-foreground mt-0.5">{m.descripcion}</p>}
            </div>
          </label>
        ))}
        {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm mt-3">{error}</div>}
        {saved && <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm mt-3">Módulos actualizados.</div>}
        <Button className="mt-4" onClick={guardar} disabled={saving}>{saving ? "Guardando..." : "Guardar módulos"}</Button>
      </CardContent>
    </Card>
  );
}

function NegocioTab({ rnc }: { rnc: string }) {
  const [values, setValues] = useState<Empresa | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    apiFetch<Empresa>(`/api/tenants/${rnc}/empresa`)
      .then(setValues)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [rnc]);

  function set<K extends keyof Empresa>(key: K, value: Empresa[K]) {
    setValues((v) => (v ? { ...v, [key]: value } : v));
    setSaved(false);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!values) return;
    setSaving(true);
    setError("");
    try {
      const data = await apiFetch<Empresa>(`/api/tenants/${rnc}/empresa`, {
        method: "PUT",
        body: JSON.stringify({
          razon_social: values.razon_social,
          nombre_comercial: values.nombre_comercial || null,
          direccion: values.direccion,
          telefono: values.telefono || null,
          correo: values.correo || null,
          logo_url: values.logo_url || null,
          ambiente_dgii: values.ambiente_dgii,
          factura_electronica_activa: values.factura_electronica_activa,
        }),
      });
      setValues(data);
      setSaved(true);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  if (loading || !values) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  return (
    <Card className="max-w-2xl">
      <CardContent className="pt-5">
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="razon_social">Razón social *</Label>
            <Input id="razon_social" required value={values.razon_social} onChange={(e) => set("razon_social", e.target.value)} />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="nombre_comercial">Nombre comercial</Label>
            <Input id="nombre_comercial" value={values.nombre_comercial || ""} onChange={(e) => set("nombre_comercial", e.target.value)} />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="direccion">Dirección *</Label>
            <Input id="direccion" required value={values.direccion} onChange={(e) => set("direccion", e.target.value)} />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label htmlFor="telefono">Teléfono</Label>
              <Input id="telefono" value={values.telefono || ""} onChange={(e) => set("telefono", e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="correo">Correo</Label>
              <Input id="correo" type="email" value={values.correo || ""} onChange={(e) => set("correo", e.target.value)} />
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="logo_url">URL del logo</Label>
            <Input id="logo_url" value={values.logo_url || ""} onChange={(e) => set("logo_url", e.target.value)} placeholder="https://..." />
          </div>
          <div className="space-y-1.5 rounded-md border border-border p-3.5">
            <label className="flex items-center gap-2 text-sm font-medium">
              <input
                type="checkbox"
                checked={values.factura_electronica_activa}
                onChange={(e) => set("factura_electronica_activa", e.target.checked)}
                className="h-4 w-4 rounded border-border"
              />
              Factura electrónica (e-CF) activada
            </label>
            {values.factura_electronica_activa && (
              <div className="pt-2 space-y-1.5">
                <Label htmlFor="ambiente">Ambiente DGII</Label>
                <Select id="ambiente" value={values.ambiente_dgii} onChange={(e) => set("ambiente_dgii", e.target.value)}>
                  <option value="TesteCF">Pruebas (TesteCF)</option>
                  <option value="CerteCF">Certificación (CerteCF)</option>
                  <option value="eCF">Producción (eCF)</option>
                </Select>
              </div>
            )}
          </div>

          {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}
          {saved && <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">Guardado correctamente.</div>}

          <Button type="submit" disabled={saving}>{saving ? "Guardando..." : "Guardar cambios"}</Button>
        </form>
      </CardContent>
    </Card>
  );
}

function DgiiTab({ rnc }: { rnc: string }) {
  const [certificado, setCertificado] = useState<CertificadoStatus | null>(null);
  const [secuencias, setSecuencias] = useState<SecuenciaNcf[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const [certFile, setCertFile] = useState<File | null>(null);
  const [certPassword, setCertPassword] = useState("");
  const [uploadingCert, setUploadingCert] = useState(false);
  const [certMsg, setCertMsg] = useState("");

  const [showForm, setShowForm] = useState(false);
  const [nueva, setNueva] = useState({ tipo_ecf: 32, prefijo: "E32", desde: "", hasta: "", fecha_vencimiento: "" });
  const [savingSecuencia, setSavingSecuencia] = useState(false);

  async function load() {
    setLoading(true);
    try {
      const [cert, secs] = await Promise.all([
        apiFetch<CertificadoStatus | null>(`/api/tenants/${rnc}/certificado`),
        apiFetch<SecuenciaNcf[]>(`/api/tenants/${rnc}/secuencias-ncf`),
      ]);
      setCertificado(cert);
      setSecuencias(secs);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rnc]);

  async function handleUploadCert(e: React.FormEvent) {
    e.preventDefault();
    if (!certFile) return;
    setUploadingCert(true);
    setCertMsg("");
    setError("");
    try {
      const p12Base64 = await fileToBase64(certFile);
      const data = await apiFetch<CertificadoStatus>(`/api/tenants/${rnc}/certificado`, {
        method: "POST",
        body: JSON.stringify({ nombre_archivo: certFile.name, p12Base64, password: certPassword }),
      });
      setCertificado(data);
      setCertFile(null);
      setCertPassword("");
      setCertMsg("Certificado cargado y verificado correctamente.");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setUploadingCert(false);
    }
  }

  async function handleCreateSecuencia(e: React.FormEvent) {
    e.preventDefault();
    setSavingSecuencia(true);
    setError("");
    try {
      await apiFetch(`/api/tenants/${rnc}/secuencias-ncf`, {
        method: "POST",
        body: JSON.stringify({
          tipo_ecf: Number(nueva.tipo_ecf),
          prefijo: nueva.prefijo,
          desde: Number(nueva.desde),
          hasta: Number(nueva.hasta),
          fecha_vencimiento: nueva.fecha_vencimiento,
        }),
      });
      setNueva({ tipo_ecf: 32, prefijo: "E32", desde: "", hasta: "", fecha_vencimiento: "" });
      setShowForm(false);
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSavingSecuencia(false);
    }
  }

  if (loading) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  return (
    <div className="space-y-6 max-w-2xl">
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card>
        <CardContent className="pt-5 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold">Certificado digital (P12)</h2>
            {certificado ? <Badge variant="success">Activo</Badge> : <Badge variant="secondary">Sin certificado</Badge>}
          </div>
          {certificado && (
            <p className="text-sm text-muted-foreground">
              {certificado.nombre_archivo} — cargado el {new Date(certificado.uploaded_at).toLocaleString("es-DO")}
            </p>
          )}
          <form onSubmit={handleUploadCert} className="space-y-3 border-t border-border pt-4">
            <div className="space-y-1.5">
              <Label htmlFor="cert_file">Archivo .p12 {certificado ? "(reemplazar)" : "*"}</Label>
              <input
                id="cert_file"
                type="file"
                accept=".p12,.pfx"
                required={!certificado}
                onChange={(e) => setCertFile(e.target.files?.[0] || null)}
                className="flex h-10 w-full rounded-md border border-border bg-background px-3 py-2 text-sm file:mr-3 file:rounded-sm file:border-0 file:bg-muted file:px-2 file:py-1 file:text-xs"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="cert_password">Contraseña del certificado *</Label>
              <Input id="cert_password" type="password" required value={certPassword} onChange={(e) => setCertPassword(e.target.value)} />
            </div>
            {certMsg && <p className="text-sm text-success">{certMsg}</p>}
            <Button type="submit" disabled={uploadingCert || !certFile}>
              <Upload className="h-4 w-4" />{uploadingCert ? "Subiendo..." : "Subir certificado"}
            </Button>
          </form>
        </CardContent>
      </Card>

      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold">Secuencias e-NCF</h2>
        <Button size="sm" onClick={() => setShowForm((s) => !s)}><Plus className="h-4 w-4" />Nueva secuencia</Button>
      </div>

      {showForm && (
        <Card>
          <CardContent className="pt-5">
            <form onSubmit={handleCreateSecuencia} className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="tipo_ecf">Tipo e-CF *</Label>
                  <Select id="tipo_ecf" required value={nueva.tipo_ecf} onChange={(e) => setNueva((v) => ({ ...v, tipo_ecf: Number(e.target.value) }))}>
                    {TIPOS_ECF.map((t) => <option key={t.value} value={t.value}>{t.label}</option>)}
                  </Select>
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="prefijo">Prefijo *</Label>
                  <Input id="prefijo" required value={nueva.prefijo} onChange={(e) => setNueva((v) => ({ ...v, prefijo: e.target.value }))} />
                </div>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="desde">Desde *</Label>
                  <Input id="desde" type="number" required value={nueva.desde} onChange={(e) => setNueva((v) => ({ ...v, desde: e.target.value }))} />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="hasta">Hasta *</Label>
                  <Input id="hasta" type="number" required value={nueva.hasta} onChange={(e) => setNueva((v) => ({ ...v, hasta: e.target.value }))} />
                </div>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="fecha_vencimiento">Fecha de vencimiento *</Label>
                <Input id="fecha_vencimiento" type="date" required value={nueva.fecha_vencimiento} onChange={(e) => setNueva((v) => ({ ...v, fecha_vencimiento: e.target.value }))} />
              </div>
              <div className="flex gap-3">
                <Button type="submit" disabled={savingSecuencia}>{savingSecuencia ? "Creando..." : "Crear secuencia"}</Button>
                <Button type="button" variant="secondary" onClick={() => setShowForm(false)}>Cancelar</Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-0">
          {secuencias.length === 0 ? (
            <p className="p-8 text-center text-sm text-muted-foreground">No hay secuencias registradas.</p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Tipo</TableHead>
                  <TableHead>Prefijo</TableHead>
                  <TableHead className="text-right">Rango</TableHead>
                  <TableHead>Vencimiento</TableHead>
                  <TableHead>Estado</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {secuencias.map((s) => (
                  <TableRow key={s.id}>
                    <TableCell>{s.tipo_ecf}</TableCell>
                    <TableCell className="font-medium">{s.prefijo}</TableCell>
                    <TableCell className="text-right tabular-nums">{s.desde} – {s.hasta}</TableCell>
                    <TableCell className="text-muted-foreground">{s.fecha_vencimiento}</TableCell>
                    <TableCell>
                      <Badge variant={s.estado === "ACTIVA" ? "success" : s.estado === "VENCIDA" || s.estado === "AGOTADA" ? "destructive" : "secondary"}>{s.estado}</Badge>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function UsuariosTab({ rnc }: { rnc: string }) {
  const [usuarios, setUsuarios] = useState<UsuarioTenant[]>([]);
  const [roles, setRoles] = useState<RolResumen[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const [showForm, setShowForm] = useState(false);
  const [nuevo, setNuevo] = useState({ nombre: "", email: "", password: "", rol_id: "" });
  const [saving, setSaving] = useState(false);

  // Última contraseña temporal generada - se muestra UNA vez, nunca se
  // vuelve a guardar en ningún lado (ni en este estado luego de cerrarla).
  const [temporal, setTemporal] = useState<{ usuarioId: string; nombre: string; password: string } | null>(null);
  const [reseteando, setReseteando] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    setError("");
    try {
      const [u, r] = await Promise.all([
        apiFetch<{ usuarios: UsuarioTenant[]; total: number }>(`/api/tenants/${rnc}/usuarios`),
        apiFetch<RolResumen[]>("/api/roles"),
      ]);
      setUsuarios(u.usuarios);
      setRoles(r);
      if (!nuevo.rol_id && r.length > 0) setNuevo((v) => ({ ...v, rol_id: r.find((x) => x.codigo === "CAJERO")?.id || r[0].id }));
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rnc]);

  async function crear(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await apiFetch(`/api/tenants/${rnc}/usuarios`, {
        method: "POST",
        body: JSON.stringify({ nombre: nuevo.nombre, email: nuevo.email, password: nuevo.password, rol: "CAJERO", rol_id: nuevo.rol_id }),
      });
      setNuevo((v) => ({ ...v, nombre: "", email: "", password: "" }));
      setShowForm(false);
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function cambiarRol(u: UsuarioTenant, rol_id: string) {
    setError("");
    try {
      await apiFetch(`/api/tenants/${rnc}/usuarios/${u.id}`, {
        method: "PUT",
        body: JSON.stringify({ nombre: u.nombre, rol: u.rol, rol_id, activo: u.activo }),
      });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function toggleActivo(u: UsuarioTenant) {
    setError("");
    try {
      if (u.activo) {
        await apiFetch(`/api/tenants/${rnc}/usuarios/${u.id}`, { method: "DELETE" });
      } else {
        await apiFetch(`/api/tenants/${rnc}/usuarios/${u.id}`, {
          method: "PUT",
          body: JSON.stringify({ nombre: u.nombre, rol: u.rol, rol_id: u.rol_id, activo: true }),
        });
      }
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function resetearPassword(u: UsuarioTenant) {
    setReseteando(u.id);
    setError("");
    try {
      const data = await apiFetch<{ password_temporal: string }>(`/api/tenants/${rnc}/usuarios/${u.id}/resetear-password`, { method: "POST" });
      setTemporal({ usuarioId: u.id, nombre: u.nombre, password: data.password_temporal });
    } catch (e: any) {
      setError(e.message);
    } finally {
      setReseteando(null);
    }
  }

  if (loading) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  return (
    <div className="space-y-4 max-w-3xl">
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      {temporal && (
        <div className="rounded-md border border-warning/30 bg-warning/10 p-4 text-sm space-y-2">
          <p className="font-medium">
            Contraseña temporal para {temporal.nombre}: <span className="font-mono text-base">{temporal.password}</span>
          </p>
          <p className="text-muted-foreground">
            Transmítesela por teléfono, WhatsApp o en persona — no se envía por correo y no se volverá a mostrar. El
            usuario deberá cambiarla al iniciar sesión.
          </p>
          <Button size="sm" variant="secondary" onClick={() => setTemporal(null)}>Entendido</Button>
        </div>
      )}

      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">Usuarios internos de este negocio (Cajero, Almacén, Contador, Admin).</p>
        <Button size="sm" onClick={() => setShowForm((s) => !s)}><Plus className="h-4 w-4" />Nuevo usuario</Button>
      </div>

      {showForm && (
        <Card>
          <CardContent className="pt-5">
            <form onSubmit={crear} className="space-y-4">
              <div className="space-y-1.5">
                <Label htmlFor="u_nombre">Nombre *</Label>
                <Input id="u_nombre" required value={nuevo.nombre} onChange={(e) => setNuevo((v) => ({ ...v, nombre: e.target.value }))} />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="u_email">Correo *</Label>
                  <Input id="u_email" type="email" required value={nuevo.email} onChange={(e) => setNuevo((v) => ({ ...v, email: e.target.value }))} />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="u_rol">Rol *</Label>
                  <Select id="u_rol" required value={nuevo.rol_id} onChange={(e) => setNuevo((v) => ({ ...v, rol_id: e.target.value }))}>
                    {roles.map((r) => <option key={r.id} value={r.id}>{r.nombre}</option>)}
                  </Select>
                </div>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="u_password">Contraseña inicial *</Label>
                <Input id="u_password" type="password" required minLength={8} value={nuevo.password} onChange={(e) => setNuevo((v) => ({ ...v, password: e.target.value }))} />
              </div>
              <div className="flex gap-3">
                <Button type="submit" disabled={saving}>{saving ? "Creando..." : "Crear usuario"}</Button>
                <Button type="button" variant="secondary" onClick={() => setShowForm(false)}>Cancelar</Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-0">
          {usuarios.length === 0 ? (
            <p className="p-8 text-center text-sm text-muted-foreground">No hay usuarios registrados.</p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Nombre</TableHead>
                  <TableHead>Correo</TableHead>
                  <TableHead>Rol</TableHead>
                  <TableHead>Estado</TableHead>
                  <TableHead className="text-right">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {usuarios.map((u) => (
                  <TableRow key={u.id}>
                    <TableCell className="font-medium">{u.nombre}</TableCell>
                    <TableCell className="text-muted-foreground">{u.email}</TableCell>
                    <TableCell>
                      <Select value={u.rol_id || ""} onChange={(e) => cambiarRol(u, e.target.value)} className="h-8 text-xs">
                        {roles.map((r) => <option key={r.id} value={r.id}>{r.nombre}</option>)}
                      </Select>
                    </TableCell>
                    <TableCell>
                      <Badge variant={u.activo ? "success" : "secondary"}>{u.activo ? "Activo" : "Inactivo"}</Badge>
                    </TableCell>
                    <TableCell className="text-right space-x-2">
                      <Button size="sm" variant="secondary" onClick={() => resetearPassword(u)} disabled={reseteando === u.id}>
                        <KeyRound className="h-3.5 w-3.5" />{reseteando === u.id ? "..." : "Resetear contraseña"}
                      </Button>
                      <Button size="sm" variant={u.activo ? "destructive" : "secondary"} onClick={() => toggleActivo(u)}>
                        {u.activo ? "Desactivar" : "Reactivar"}
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
