"use client";

import { useState } from "react";
import { Download } from "lucide-react";
import { Button, Card, CardContent, Input, Label } from "@repo/ui";

export function ReporteTxt({ titulo, descripcion, endpoint, filePrefix }: { titulo: string; descripcion: string; endpoint: string; filePrefix: string }) {
  const now = new Date();
  const [period, setPeriod] = useState(`${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, "0")}`);
  const [texto, setTexto] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function handleGenerar(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError("");
    setTexto("");
    try {
      const token = localStorage.getItem("token");
      const res = await fetch(`${endpoint}?period=${period}`, { headers: { Authorization: `Bearer ${token}` } });
      const body = await res.text();
      if (!res.ok) {
        const parsed = JSON.parse(body);
        throw new Error(parsed.error || "Error generando el reporte");
      }
      setTexto(body);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  function handleDescargar() {
    const blob = new Blob([texto], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${filePrefix}${period}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">{titulo}</h1>
        <p className="text-sm text-muted-foreground mt-1">{descripcion}</p>
      </div>

      <Card className="max-w-md">
        <CardContent className="pt-5">
          <form onSubmit={handleGenerar} className="flex items-end gap-3">
            <div className="flex-1 space-y-1.5">
              <Label htmlFor="period">Período (AAAAMM)</Label>
              <Input id="period" value={period} onChange={(e) => setPeriod(e.target.value)} placeholder="202607" required maxLength={6} />
            </div>
            <Button type="submit" disabled={loading}>{loading ? "Generando..." : "Generar"}</Button>
          </form>
          {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm mt-3">{error}</div>}
        </CardContent>
      </Card>

      {texto && (
        <Card>
          <CardContent className="pt-5">
            <div className="flex justify-between items-center mb-2">
              <p className="text-sm font-semibold">Archivo generado</p>
              <Button size="sm" variant="secondary" onClick={handleDescargar}><Download className="h-4 w-4" />Descargar .txt</Button>
            </div>
            <pre className="text-xs font-mono bg-muted/50 rounded-md p-3 overflow-x-auto whitespace-pre-wrap">{texto}</pre>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
