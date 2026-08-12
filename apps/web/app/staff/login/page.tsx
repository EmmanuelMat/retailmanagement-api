"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { ShieldCheck } from "lucide-react";
import { Button, Card, CardContent, Input, Label } from "@repo/ui";
import { ApiError } from "@/lib/staff-api";

export default function LoginPage() {
  const router = useRouter();
  const [secret, setSecret] = useState("");
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState("");

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setChecking(true);
    setError("");
    try {
      const res = await fetch("/api/staff/modulos", { headers: { "X-Vendor-Secret": secret } });
      if (!res.ok) {
        throw new ApiError("Clave incorrecta", res.status);
      }
      localStorage.setItem("vendorSecret", secret);
      router.push("/tenants" as any);
    } catch (e: any) {
      setError(e.message || "Clave incorrecta");
    } finally {
      setChecking(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <Card className="w-full max-w-sm">
        <CardContent className="pt-6">
          <div className="flex items-center gap-2.5 mb-6">
            <div className="h-9 w-9 rounded-md bg-primary text-primary-foreground flex items-center justify-center shrink-0">
              <ShieldCheck className="h-4.5 w-4.5" />
            </div>
            <div>
              <p className="font-semibold text-[15px] leading-tight">Staff</p>
              <p className="text-[11px] text-muted-foreground leading-none">Colmado POS — panel interno</p>
            </div>
          </div>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="secret">Clave de vendedor</Label>
              <Input
                id="secret"
                type="password"
                required
                autoFocus
                value={secret}
                onChange={(e) => setSecret(e.target.value)}
                placeholder="X-Vendor-Secret"
              />
            </div>
            {error && <p className="text-sm text-destructive">{error}</p>}
            <Button type="submit" className="w-full" disabled={checking || !secret.trim()}>
              {checking ? "Verificando..." : "Entrar"}
            </Button>
          </form>
          <p className="text-[11px] text-muted-foreground mt-5 text-center">
            No es el login de un cliente. Esta herramienta usa la misma clave que ya sirve para activar licencias.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
