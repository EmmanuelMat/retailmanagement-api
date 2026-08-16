"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { Building2, LayoutGrid, LogOut, ShieldCheck, Users } from "lucide-react";

const NAV = [
  { label: "Negocios", href: "/tenants", icon: Building2 },
  { label: "Catálogo de módulos", href: "/modulos", icon: LayoutGrid },
  { label: "Roles", href: "/roles", icon: Users },
];

export default function StaffLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const secret = localStorage.getItem("vendorSecret");
    if (!secret) {
      router.replace("/login");
      return;
    }
    setReady(true);
  }, [router]);

  function cerrarSesion() {
    localStorage.removeItem("vendorSecret");
    router.push("/login");
  }

  if (!ready) return null;

  return (
    <div className="h-screen overflow-hidden bg-background">
      <div className="flex h-full">
        <aside className="hidden lg:flex lg:flex-col w-[232px] shrink-0 border-r border-border bg-surface h-full overflow-y-auto">
          <Link href={"/tenants" as any} className="flex items-center gap-2.5 px-5 h-16 border-b border-border shrink-0">
            <div className="h-8 w-8 rounded-md bg-primary text-primary-foreground flex items-center justify-center shrink-0">
              <ShieldCheck className="h-4 w-4" />
            </div>
            <div className="min-w-0">
              <p className="font-semibold text-[15px] leading-tight">Staff</p>
              <p className="text-[10px] text-muted-foreground leading-none">Colmado POS</p>
            </div>
          </Link>
          <nav className="flex-1 overflow-y-auto py-4 px-3 space-y-0.5">
            {NAV.map((item) => {
              const active = pathname === item.href || pathname.startsWith(item.href + "/");
              const Icon = item.icon;
              return (
                <Link
                  key={item.href}
                  href={item.href as any}
                  className={`relative flex items-center gap-2.5 px-2.5 py-1.5 rounded-md text-sm transition-colors ${
                    active ? "bg-primary/10 text-primary font-medium" : "text-foreground/75 hover:bg-muted hover:text-foreground"
                  }`}
                >
                  {active && <span className="absolute left-0 top-1.5 bottom-1.5 w-[3px] rounded-full bg-primary" />}
                  <Icon className="h-4 w-4 shrink-0" />
                  {item.label}
                </Link>
              );
            })}
          </nav>
          <div className="border-t border-border p-3">
            <button
              onClick={cerrarSesion}
              className="w-full flex items-center gap-2.5 px-2.5 py-2 rounded-md text-sm text-foreground/75 hover:bg-muted hover:text-foreground transition-colors"
            >
              <LogOut className="h-4 w-4" />
              Salir
            </button>
          </div>
        </aside>

        <div className="flex-1 min-w-0 h-full overflow-y-auto">
          <main className="p-6 max-w-5xl mx-auto">{children}</main>
        </div>
      </div>
    </div>
  );
}
