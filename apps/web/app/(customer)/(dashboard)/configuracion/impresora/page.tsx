"use client";

import { useEffect, useState } from "react";
import { Wifi, WifiOff } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label, Select } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface ImpresoraConfig {
  ip: string | null;
  puerto: number;
  ancho_mm: number;
  copias: number;
}

const EMPTY: ImpresoraConfig = { ip: "", puerto: 9100, ancho_mm: 80, copias: 1 };

export default function ImpresoraPage() {
  const [values, setValues] = useState<ImpresoraConfig>(EMPTY);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ alcanzable: boolean; mensaje: string } | null>(null);

  useEffect(() => {
    apiFetch<ImpresoraConfig>("/api/config/impresora")
      .then((data) => setValues({ ...data, ip: data.ip || "" }))
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  function set<K extends keyof ImpresoraConfig>(key: K, value: string | number) {
    setValues((v) => ({ ...v, [key]: value }));
    setSaved(false);
    setTestResult(null);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      const data = await apiFetch<ImpresoraConfig>("/api/config/impresora", {
        method: "PUT",
        body: JSON.stringify({ ip: values.ip || null, puerto: Number(values.puerto), ancho_mm: Number(values.ancho_mm), copias: Number(values.copias) }),
      });
      setValues({ ...data, ip: data.ip || "" });
      setSaved(true);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleTest() {
    setTesting(true);
    setTestResult(null);
    setError("");
    try {
      const result = await apiFetch<{ alcanzable: boolean; mensaje: string }>("/api/config/impresora/test", { method: "POST" });
      setTestResult(result);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setTesting(false);
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Impresora</h1>
        <p className="text-sm text-muted-foreground mt-1">Configuración de la impresora térmica de tickets en red.</p>
      </div>

      {loading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : (
        <Card className="max-w-4xl">
          <CardContent className="pt-5">
            <form onSubmit={handleSubmit} className="space-y-5">
              <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div className="space-y-1.5 md:col-span-2">
                  <Label htmlFor="ip">Dirección IP</Label>
                  <Input id="ip" value={values.ip || ""} onChange={(e) => set("ip", e.target.value)} placeholder="192.168.1.50" />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="puerto">Puerto</Label>
                  <Input id="puerto" type="number" value={values.puerto} onChange={(e) => set("puerto", e.target.value)} />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="copias">Copias por venta</Label>
                  <Input id="copias" type="number" min={1} max={5} value={values.copias} onChange={(e) => set("copias", e.target.value)} />
                </div>
              </div>
              <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="ancho_mm">Ancho de papel</Label>
                  <Select id="ancho_mm" value={values.ancho_mm} onChange={(e) => set("ancho_mm", Number(e.target.value))}>
                    <option value={58}>58mm</option>
                    <option value={80}>80mm</option>
                  </Select>
                </div>
              </div>

              {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}
              {saved && <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">Guardado correctamente.</div>}

              <div className="flex items-center gap-3">
                <Button type="submit" disabled={saving}>{saving ? "Guardando..." : "Guardar cambios"}</Button>
                <Button type="button" variant="secondary" onClick={handleTest} disabled={testing || !values.ip}>
                  {testing ? "Probando..." : "Probar conexión"}
                </Button>
                {testResult && (
                  <Badge variant={testResult.alcanzable ? "success" : "destructive"}>
                    {testResult.alcanzable ? <Wifi className="h-3 w-3 mr-1 inline" /> : <WifiOff className="h-3 w-3 mr-1 inline" />}
                    {testResult.mensaje}
                  </Badge>
                )}
              </div>
            </form>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
