"use client";
import { useState } from "react";
import Link from "next/link";
import { Store, Mail } from "lucide-react";
import { Button, Card, Input, Label } from "@repo/ui";

export default function OlvidePasswordPage() {
  const [loading, setLoading] = useState(false);
  const [enviado, setEnviado] = useState(false);

  async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setLoading(true);
    const form = new FormData(e.currentTarget);
    const email = form.get("email") as string;
    const rnc = form.get("rnc") as string;
    try {
      await fetch("/api/auth/forgot-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: email.toLowerCase().trim(), rnc: rnc ? rnc.replace(/-/g, "").trim() || undefined : undefined }),
      });
    } catch {
      // El mensaje es el mismo se haya podido contactar el servidor o no -
      // no hay nada útil que distinguir aquí para el usuario.
    } finally {
      // Siempre se muestra el mismo resultado, exista o no la cuenta - ver
      // FORGOT_PASSWORD_MENSAJE en el core.
      setEnviado(true);
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
          {enviado ? (
            <div className="text-center space-y-3">
              <div className="mx-auto h-12 w-12 rounded-full bg-primary/10 text-primary flex items-center justify-center">
                <Mail className="h-6 w-6" />
              </div>
              <h2 className="font-bold text-xl font-serif">Revisa tu correo</h2>
              <p className="text-sm text-muted-foreground">
                Si el correo existe, te enviamos un enlace para restablecer tu contraseña. Vence en 30 minutos.
              </p>
              <Link href="/login" className="text-sm text-primary hover:underline block pt-2">Volver a iniciar sesión</Link>
            </div>
          ) : (
            <>
              <h2 className="font-bold text-2xl font-serif">¿Olvidaste tu contraseña?</h2>
              <p className="text-sm text-muted-foreground mt-1.5">Te enviaremos un enlace para restablecerla.</p>

              <form onSubmit={handleSubmit} className="mt-7 space-y-4">
                <div className="space-y-1.5">
                  <Label htmlFor="rnc">RNC de la empresa (opcional)</Label>
                  <Input id="rnc" name="rnc" placeholder="130793752" />
                  <p className="text-xs text-muted-foreground">Solo necesario si usas el mismo correo en varios negocios.</p>
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="email">Correo electrónico</Label>
                  <Input id="email" name="email" type="email" required placeholder="juan@colmadoelsol.do" />
                </div>

                <Button type="submit" disabled={loading} className="w-full" size="lg">
                  {loading ? "Enviando..." : "Enviar enlace"}
                </Button>

                <div className="text-center">
                  <Link href="/login" className="text-sm text-primary hover:underline">Volver a iniciar sesión</Link>
                </div>
              </form>
            </>
          )}
        </Card>
      </div>
    </div>
  );
}
