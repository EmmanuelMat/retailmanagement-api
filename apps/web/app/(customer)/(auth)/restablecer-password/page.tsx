"use client";
import { useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { Store, CheckCircle2 } from "lucide-react";
import { Button, Card, Input, Label } from "@repo/ui";

export default function RestablecerPasswordPage() {
  const searchParams = useSearchParams();
  const token = searchParams.get("token") || "";
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [listo, setListo] = useState(false);

  async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const form = new FormData(e.currentTarget);
    const password = form.get("password") as string;
    const confirmar = form.get("confirmar") as string;
    if (password !== confirmar) {
      setError("Las contraseñas no coinciden");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const res = await fetch("/api/auth/reset-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token, new_password: password }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "No se pudo restablecer la contraseña");
      setListo(true);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center p-6 bg-muted/30">
      <div className="w-full max-w-[420px]">
        <Link href="/" className="flex items-center gap-3 mb-6 justify-center">
          <div className="h-10 w-10 rounded-md bg-primary text-primary-foreground flex items-center justify-center">
            <Store className="h-5 w-5" />
          </div>
          <h1 className="font-bold text-[17px]">Colmado POS</h1>
        </Link>

        <Card className="p-8">
          {listo ? (
            <div className="text-center space-y-3">
              <div className="mx-auto h-12 w-12 rounded-full bg-success/10 text-success flex items-center justify-center">
                <CheckCircle2 className="h-6 w-6" />
              </div>
              <h2 className="font-bold text-xl font-serif">Contraseña actualizada</h2>
              <p className="text-sm text-muted-foreground">Ya puedes iniciar sesión con tu nueva contraseña.</p>
              <Link href="/login"><Button className="w-full mt-2">Ir a iniciar sesión</Button></Link>
            </div>
          ) : !token ? (
            <div className="text-center space-y-3">
              <h2 className="font-bold text-xl font-serif">Enlace inválido</h2>
              <p className="text-sm text-muted-foreground">Este enlace no incluye un token. Solicita uno nuevo.</p>
              <Link href="/olvide-password" className="text-sm text-primary hover:underline block pt-2">Solicitar enlace</Link>
            </div>
          ) : (
            <>
              <h2 className="font-bold text-2xl font-serif">Nueva contraseña</h2>
              <p className="text-sm text-muted-foreground mt-1.5">Este enlace vence 30 minutos después de haberlo solicitado.</p>

              <form onSubmit={handleSubmit} className="mt-7 space-y-4">
                <div className="space-y-1.5">
                  <Label htmlFor="password">Nueva contraseña</Label>
                  <Input id="password" name="password" type="password" required minLength={8} placeholder="Mínimo 8 caracteres" />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="confirmar">Confirmar contraseña</Label>
                  <Input id="confirmar" name="confirmar" type="password" required minLength={8} />
                </div>

                {error && (
                  <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">
                    {error}
                  </div>
                )}

                <Button type="submit" disabled={loading} className="w-full" size="lg">
                  {loading ? "Guardando..." : "Restablecer contraseña"}
                </Button>
              </form>
            </>
          )}
        </Card>
      </div>
    </div>
  );
}
