"use client";

import { useEffect, useState } from "react";
import { Button, Card, CardContent, Input, Label, Select } from "@repo/ui";
import { apiFetch } from "@/lib/api";

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

const EMPTY: Empresa = {
  rnc: "",
  razon_social: "",
  nombre_comercial: "",
  direccion: "",
  telefono: "",
  correo: "",
  logo_url: "",
  ambiente_dgii: "TesteCF",
  factura_electronica_activa: true,
};

export default function EmpresaPage() {
  const [values, setValues] = useState<Empresa>(EMPTY);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const [respaldando, setRespaldando] = useState(false);
  const [errorRespaldo, setErrorRespaldo] = useState("");

  useEffect(() => {
    apiFetch<Empresa>("/api/config/empresa")
      .then((data) => setValues({ ...data, nombre_comercial: data.nombre_comercial || "", telefono: data.telefono || "", correo: data.correo || "", logo_url: data.logo_url || "" }))
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  function set<K extends keyof Empresa>(key: K, value: string) {
    setValues((v) => ({ ...v, [key]: value }));
    setSaved(false);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      const data = await apiFetch<Empresa>("/api/config/empresa", {
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
      const cambioFacturaElectronica = values.factura_electronica_activa !== data.factura_electronica_activa;
      setValues({ ...data, nombre_comercial: data.nombre_comercial || "", telefono: data.telefono || "", correo: data.correo || "", logo_url: data.logo_url || "" });
      setSaved(true);
      // El sidebar (layout.tsx) lee el tenant de localStorage, escrito solo
      // al iniciar sesión - sin esto, un cambio aquí (p.ej. activar/desactivar
      // e-CF) no se reflejaba hasta cerrar sesión y volver a entrar.
      try {
        const existente = JSON.parse(localStorage.getItem("tenant") || "{}");
        localStorage.setItem("tenant", JSON.stringify({ ...existente, ...data }));
      } catch {}
      // El layout ya está montado y no vuelve a leer localStorage solo, así
      // que el sidebar (qué secciones DGII muestra) no se actualizaría solo -
      // recargar aquí es más simple y confiable que levantar el tenant a un
      // contexto compartido. Solo cuando este campo realmente cambió, para no
      // interrumpir un guardado normal de datos de contacto.
      if (cambioFacturaElectronica) {
        setTimeout(() => window.location.reload(), 600);
      }
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function descargarRespaldo() {
    setRespaldando(true);
    setErrorRespaldo("");
    try {
      const token = localStorage.getItem("token");
      const res = await fetch("/api/backup/descargar", {
        headers: token ? { Authorization: `Bearer ${token}` } : {},
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        throw new Error(data?.error || `Error ${res.status}`);
      }
      const blob = await res.blob();
      const disposition = res.headers.get("content-disposition") || "";
      const match = disposition.match(/filename="?([^"]+)"?/);
      const filename = match?.[1] || "backup.dump";
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e: any) {
      setErrorRespaldo(e.message || "No se pudo generar el respaldo");
    } finally {
      setRespaldando(false);
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Mi negocio</h1>
        <p className="text-sm text-muted-foreground mt-1">Datos fiscales y de contacto usados en facturas y reportes DGII.</p>
      </div>

      {loading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : (
        <Card className="max-w-4xl">
          <CardContent className="pt-5">
            <form onSubmit={handleSubmit} className="space-y-5">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="rnc">RNC</Label>
                  <Input id="rnc" value={values.rnc} disabled />
                </div>
                <div className="space-y-1.5 md:col-span-2">
                  <Label htmlFor="razon_social">Razón social *</Label>
                  <Input id="razon_social" required value={values.razon_social} onChange={(e) => set("razon_social", e.target.value)} />
                </div>
              </div>

              <div className="space-y-1.5 rounded-md border border-border p-3.5">
                <Label>¿Tu negocio factura electrónicamente con la DGII?</Label>
                <div className="grid grid-cols-2 gap-2 max-w-sm">
                  <button
                    type="button"
                    onClick={() => {
                      setValues((v) => ({ ...v, factura_electronica_activa: true }));
                      setSaved(false);
                    }}
                    className={`h-10 rounded-md border text-sm font-medium transition-colors ${
                      values.factura_electronica_activa ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:bg-muted"
                    }`}
                  >
                    Sí
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setValues((v) => ({ ...v, factura_electronica_activa: false }));
                      setSaved(false);
                    }}
                    className={`h-10 rounded-md border text-sm font-medium transition-colors ${
                      !values.factura_electronica_activa ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:bg-muted"
                    }`}
                  >
                    No
                  </button>
                </div>
                {values.factura_electronica_activa && (
                  <div className="pt-2 space-y-1.5 max-w-sm">
                    <Label htmlFor="ambiente">Ambiente DGII *</Label>
                    <Select id="ambiente" required value={values.ambiente_dgii} onChange={(e) => set("ambiente_dgii", e.target.value)}>
                      <option value="TesteCF">Pruebas (TesteCF)</option>
                      <option value="CerteCF">Certificación (CerteCF)</option>
                      <option value="eCF">Producción (eCF)</option>
                    </Select>
                  </div>
                )}
              </div>

              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div className="space-y-1.5 md:col-span-2">
                  <Label htmlFor="nombre_comercial">Nombre comercial</Label>
                  <Input id="nombre_comercial" value={values.nombre_comercial || ""} onChange={(e) => set("nombre_comercial", e.target.value)} />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="telefono">Teléfono</Label>
                  <Input id="telefono" value={values.telefono || ""} onChange={(e) => set("telefono", e.target.value)} placeholder="809-000-0000" />
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div className="space-y-1.5 md:col-span-2">
                  <Label htmlFor="direccion">Dirección *</Label>
                  <Input id="direccion" required value={values.direccion} onChange={(e) => set("direccion", e.target.value)} />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="correo">Correo</Label>
                  <Input id="correo" type="email" value={values.correo || ""} onChange={(e) => set("correo", e.target.value)} />
                </div>
              </div>

              <div className="space-y-1.5 max-w-md">
                <Label htmlFor="logo_url">URL del logo</Label>
                <Input id="logo_url" value={values.logo_url || ""} onChange={(e) => set("logo_url", e.target.value)} placeholder="https://..." />
              </div>

              {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}
              {saved && <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">Guardado correctamente.</div>}

              <Button type="submit" disabled={saving}>{saving ? "Guardando..." : "Guardar cambios"}</Button>
            </form>
          </CardContent>
        </Card>
      )}

      <Card className="max-w-4xl">
        <CardContent className="pt-5 space-y-3">
          <div>
            <p className="text-sm font-semibold">Respaldo de datos</p>
            <p className="text-sm text-muted-foreground mt-1">
              Todo tu negocio corre en esta computadora — sin nube de por medio. Se genera un respaldo
              automático cada 24 horas, pero también puedes descargar uno ahora mismo cuando quieras.
            </p>
          </div>
          {errorRespaldo && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{errorRespaldo}</div>}
          <Button type="button" variant="outline" onClick={descargarRespaldo} disabled={respaldando}>
            {respaldando ? "Generando respaldo..." : "Descargar respaldo ahora"}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
