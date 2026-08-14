"use client";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Store } from "lucide-react";
import { Button, Card, Input, Label } from "@repo/ui";

/**
 * Único destino alcanzable cuando `usuario.must_change_password` es true
 * (staff reseteó la contraseña sin correo, ver apps/web/app/staff) - el
 * layout del dashboard redirige aquí antes de dejar pasar a cualquier otra
 * pantalla (ver `must_change_password_guard` en el core, que hace lo mismo
 * en el backend).
 */
export default function CambiarPasswordPage() {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!localStorage.getItem("token")) router.replace("/login");
  }, [router]);

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
      const res = await fetch("/api/auth/set-new-password", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${localStorage.getItem("token") || ""}`,
        },
        body: JSON.stringify({ new_password: password }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "No se pudo actualizar la contraseña");

      localStorage.setItem("token", data.token);
      localStorage.setItem("usuario", JSON.stringify(data.usuario));
      localStorage.setItem("tenant", JSON.stringify(data.tenant));
      document.cookie = `token=${data.token}; path=/; max-age=43200`;

      window.location.href = "/pos";
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center p-6 bg-muted/30">
      <div className="w-full max-w-[420px]">
        <div className="flex items-center gap-3 mb-6 justify-center">
          <div className="h-10 w-10 rounded-md bg-primary text-primary-foreground flex items-center justify-center">
            <Store className="h-5 w-5" />
          </div>
          <h1 className="font-bold text-[17px]">Colmado POS</h1>
        </div>

        <Card className="p-8">
          <h2 className="font-bold text-2xl font-serif">Establece una nueva contraseña</h2>
          <p className="text-sm text-muted-foreground mt-1.5">
            Tu contraseña fue restablecida. Debes crear una nueva antes de continuar.
          </p>

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
              {loading ? "Guardando..." : "Guardar y continuar"}
            </Button>
          </form>
        </Card>
      </div>
    </div>
  );
}
