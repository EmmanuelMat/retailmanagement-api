"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { FileText, Plus, ShieldCheck, ShieldX, Upload } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label, Select, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@repo/ui";
import { apiFetch } from "@/lib/api";

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

interface CertificadoStatus {
  id: string;
  nombre_archivo: string;
  activo: boolean;
  uploaded_at: string;
}

const TIPOS_ECF = [
  { value: 31, label: "31 · Crédito Fiscal" },
  { value: 32, label: "32 · Consumo" },
  { value: 33, label: "33 · Nota de Débito" },
  { value: 34, label: "34 · Nota de Crédito" },
  { value: 41, label: "41 · Compras" },
  { value: 43, label: "43 · Gastos Menores" },
  { value: 44, label: "44 · Regímenes Especiales" },
  { value: 45, label: "45 · Gubernamental" },
  { value: 46, label: "46 · Exportaciones" },
  { value: 47, label: "47 · Pagos al Exterior" },
];

const EMPTY_SECUENCIA = { tipo_ecf: 32, prefijo: "E32", desde: "", hasta: "", fecha_vencimiento: "" };

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      resolve(result.split(",")[1] || "");
    };
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

export default function DgiiConfigPage() {
  const [secuencias, setSecuencias] = useState<SecuenciaNcf[]>([]);
  const [certificado, setCertificado] = useState<CertificadoStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const [showForm, setShowForm] = useState(false);
  const [nueva, setNueva] = useState(EMPTY_SECUENCIA);
  const [savingSecuencia, setSavingSecuencia] = useState(false);

  const [certFile, setCertFile] = useState<File | null>(null);
  const [certPassword, setCertPassword] = useState("");
  const [uploadingCert, setUploadingCert] = useState(false);
  const [certMsg, setCertMsg] = useState("");

  async function load() {
    setLoading(true);
    try {
      const [secData, certData] = await Promise.all([
        apiFetch<{ secuencias: SecuenciaNcf[] }>("/api/config/secuencias-ncf"),
        apiFetch<{ certificado: CertificadoStatus | null }>("/api/config/certificado"),
      ]);
      setSecuencias(secData.secuencias);
      setCertificado(certData.certificado);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function handleCreateSecuencia(e: React.FormEvent) {
    e.preventDefault();
    setSavingSecuencia(true);
    setError("");
    try {
      await apiFetch("/api/config/secuencias-ncf", {
        method: "POST",
        body: JSON.stringify({
          tipo_ecf: Number(nueva.tipo_ecf),
          prefijo: nueva.prefijo,
          desde: Number(nueva.desde),
          hasta: Number(nueva.hasta),
          fecha_vencimiento: nueva.fecha_vencimiento,
        }),
      });
      setNueva(EMPTY_SECUENCIA);
      setShowForm(false);
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSavingSecuencia(false);
    }
  }

  async function handleSetEstado(id: string, estado: string) {
    try {
      await apiFetch(`/api/config/secuencias-ncf/${id}/estado`, { method: "PUT", body: JSON.stringify({ estado }) });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function handleUploadCert(e: React.FormEvent) {
    e.preventDefault();
    if (!certFile) return;
    setUploadingCert(true);
    setCertMsg("");
    setError("");
    try {
      const p12Base64 = await fileToBase64(certFile);
      const data = await apiFetch<CertificadoStatus>("/api/config/certificado", {
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

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Configuración DGII</h1>
          <p className="text-sm text-muted-foreground mt-1">Secuencias de e-NCF y certificado digital para firmar los e-CF.</p>
        </div>
        <Link href="/configuracion/dgii/documentos">
          <Button variant="secondary"><FileText className="h-4 w-4" />Documentos e-CF</Button>
        </Link>
      </div>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card className="max-w-2xl">
        <CardContent className="pt-5 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold">Certificado digital (P12)</h2>
            {certificado ? (
              <Badge variant="success"><ShieldCheck className="h-3 w-3 mr-1 inline" />Activo</Badge>
            ) : (
              <Badge variant="secondary"><ShieldX className="h-3 w-3 mr-1 inline" />Sin certificado</Badge>
            )}
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
        <Button onClick={() => setShowForm((s) => !s)}><Plus className="h-4 w-4" />Nueva secuencia</Button>
      </div>

      {showForm && (
        <Card className="max-w-2xl">
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
                  <Input id="prefijo" required value={nueva.prefijo} onChange={(e) => setNueva((v) => ({ ...v, prefijo: e.target.value }))} placeholder="E32" />
                </div>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="desde">Desde *</Label>
                  <Input id="desde" type="number" required value={nueva.desde} onChange={(e) => setNueva((v) => ({ ...v, desde: e.target.value }))} placeholder="1" />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="hasta">Hasta *</Label>
                  <Input id="hasta" type="number" required value={nueva.hasta} onChange={(e) => setNueva((v) => ({ ...v, hasta: e.target.value }))} placeholder="50000" />
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
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : secuencias.length === 0 ? (
            <p className="p-8 text-center text-sm text-muted-foreground">No hay secuencias registradas.</p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Tipo</TableHead>
                  <TableHead>Prefijo</TableHead>
                  <TableHead className="text-right">Rango</TableHead>
                  <TableHead className="text-right">Próximo</TableHead>
                  <TableHead className="text-right">Restante</TableHead>
                  <TableHead>Vencimiento</TableHead>
                  <TableHead>Estado</TableHead>
                  <TableHead className="w-28 text-right">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {secuencias.map((s) => {
                  const rango = s.hasta - s.desde + 1;
                  const restante = s.hasta - s.proximo + 1;
                  const bajo = s.estado === "ACTIVA" && (restante <= 50 || restante / rango <= 0.1);
                  return (
                    <TableRow key={s.id}>
                      <TableCell>{s.tipo_ecf}</TableCell>
                      <TableCell className="font-medium">{s.prefijo}</TableCell>
                      <TableCell className="text-right tabular-nums">{s.desde} – {s.hasta}</TableCell>
                      <TableCell className="text-right tabular-nums">{s.proximo}</TableCell>
                      <TableCell className="text-right tabular-nums">
                        {bajo ? <Badge variant="warning">{restante}</Badge> : restante}
                      </TableCell>
                      <TableCell className="text-muted-foreground">{s.fecha_vencimiento}</TableCell>
                      <TableCell>
                        <Badge variant={s.estado === "ACTIVA" ? "success" : s.estado === "VENCIDA" || s.estado === "AGOTADA" ? "destructive" : "secondary"}>{s.estado}</Badge>
                      </TableCell>
                      <TableCell className="text-right">
                        {s.estado === "ACTIVA" && (
                          <Button size="sm" variant="ghost" onClick={() => handleSetEstado(s.id, "VENCIDA")}>Cancelar</Button>
                        )}
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
