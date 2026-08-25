"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import {
  LayoutDashboard,
  ShoppingCart,
  Tags,
  Package,
  Users,
  Truck,
  ShoppingBag,
  BookOpen,
  Wallet,
  Landmark,
  Briefcase,
  HandCoins,
  FileBarChart,
  ShieldCheck,
  Store,
  LogOut,
  Building2,
  UserCog,
  Printer,
  History,
  AlertTriangle,
  FileText,
  PackageCheck,
  ChevronDown,
  ChevronRight,
  Wrench,
  ClipboardList,
} from "lucide-react";
import { isRouteAllowed, isDgiiRoute, isModuloAllowed, isModuloActivo } from "@/lib/roles";
import { apiFetch } from "@/lib/api";
import AiChatWidget from "./ai-chat-widget";

type NavItem = { label: string; href: string; icon: React.ComponentType<{ className?: string }> };
type NavGroup = { title: string; items: NavItem[] };

const NAV: NavGroup[] = [
  { title: "Principal", items: [{ label: "Dashboard", href: "/dashboard", icon: LayoutDashboard }] },
  {
    title: "Ventas",
    items: [
      { label: "Punto de Venta", href: "/pos", icon: ShoppingCart },
      { label: "Ventas", href: "/ventas", icon: FileBarChart },
      { label: "Cotizaciones", href: "/cotizaciones", icon: FileText },
      { label: "Conduces", href: "/conduces", icon: PackageCheck },
      { label: "Clientes", href: "/clientes", icon: Users },
    ],
  },
  {
    title: "Inventario",
    items: [
      { label: "Productos", href: "/inventario/productos", icon: Package },
      { label: "Categorías", href: "/inventario/categorias", icon: Tags },
      { label: "Movimientos", href: "/inventario/movimientos", icon: Package },
    ],
  },
  {
    title: "Servicios",
    items: [
      { label: "Órdenes de Servicio", href: "/ordenes-servicio", icon: Wrench },
    ],
  },
  {
    title: "Compras",
    items: [
      { label: "Compras", href: "/compras", icon: ShoppingBag },
      { label: "Órdenes de Compra", href: "/ordenes-compra", icon: ClipboardList },
      { label: "Proveedores", href: "/proveedores", icon: Truck },
    ],
  },
  {
    title: "Finanzas",
    items: [
      { label: "Contabilidad", href: "/contabilidad", icon: BookOpen },
      { label: "Caja", href: "/caja", icon: Wallet },
      { label: "Bancos", href: "/bancos", icon: Landmark },
    ],
  },
  {
    title: "Nómina",
    items: [
      { label: "Nómina", href: "/nomina", icon: Briefcase },
      { label: "Adelantos", href: "/nomina/adelantos", icon: HandCoins },
    ],
  },
  {
    title: "Reportes y Config",
    items: [
      { label: "Reportes DGII", href: "/reportes/dgii", icon: FileBarChart },
      { label: "Config DGII", href: "/configuracion/dgii", icon: ShieldCheck },
      { label: "Auditoría", href: "/reportes/auditoria", icon: History },
      { label: "Mi negocio", href: "/configuracion/empresa", icon: Building2 },
      { label: "Usuarios", href: "/configuracion/usuarios", icon: UserCog },
      { label: "Impresora", href: "/configuracion/impresora", icon: Printer },
    ],
  },
];

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const [tenant, setTenant] = useState<{ razon_social?: string; nombre_comercial?: string | null; logo_url?: string | null; factura_electronica_activa?: boolean; tipo_negocio?: "COLMADO" | "SERVICIOS" } | null>(null);
  const [usuario, setUsuario] = useState<{ nombre?: string; rol?: string } | null>(null);
  const [licencia, setLicencia] = useState<{ status: "trial" | "active" | "expired"; dias_restantes: number } | null>(null);
  const [modulosActivos, setModulosActivos] = useState<Set<string> | null>(null);
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(() => {
    if (typeof window === "undefined") return new Set();
    try {
      const raw = localStorage.getItem("nav_collapsed_groups");
      return raw ? new Set(JSON.parse(raw)) : new Set();
    } catch {
      return new Set();
    }
  });

  function toggleGroup(title: string) {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(title)) next.delete(title);
      else next.add(title);
      try {
        localStorage.setItem("nav_collapsed_groups", JSON.stringify([...next]));
      } catch {}
      return next;
    });
  }

  // Acento de marca por tipo de negocio (ver globals.css `[data-negocio]`) -
  // COLMADO usa el "Sello" de siempre sin overrides; solo SERVICIOS cambia
  // el hue de --primary/--accent. Puesto en <html> (no solo en este layout)
  // para que /imprimir y /staff nunca lo hereden sin querer.
  useEffect(() => {
    if (tenant?.tipo_negocio) {
      document.documentElement.setAttribute("data-negocio", tenant.tipo_negocio);
    }
    return () => {
      document.documentElement.removeAttribute("data-negocio");
    };
  }, [tenant?.tipo_negocio]);

  useEffect(() => {
    try {
      const t = localStorage.getItem("tenant");
      if (t) setTenant(JSON.parse(t));
      const u = localStorage.getItem("usuario");
      if (u) setUsuario(JSON.parse(u));
    } catch {}
    apiFetch<{ status: "trial" | "active" | "expired"; dias_restantes: number }>("/api/license/status")
      .then(setLicencia)
      .catch(() => {});
    apiFetch<{ codigo: string; activo: boolean }[]>("/api/tenants/me/modulos")
      .then((modulos) => setModulosActivos(new Set(modulos.filter((m) => m.activo).map((m) => m.codigo))))
      .catch(() => {});
  }, []);

  // El ícono de la pestaña del navegador sigue al negocio con sesión activa,
  // no al producto (Colmado POS) - cada tenant ve su propio logo de Mi
  // negocio en la pestaña, no uno genérico compartido entre todos.
  useEffect(() => {
    if (!tenant?.logo_url) return;
    let cancelled = false;

    function setFavicon(href: string) {
      let link = document.querySelector<HTMLLinkElement>("link[rel='icon']");
      if (!link) {
        link = document.createElement("link");
        link.rel = "icon";
        document.head.appendChild(link);
      }
      link.href = href;
    }

    // El logo subido puede venir en cualquier proporción/tamaño - un logo
    // ancho o muy grande se ve mal aplastado en los ~16-32px de una pestaña.
    // Se dibuja centrado y a escala ("contain") sobre un lienzo cuadrado
    // antes de usarlo como ícono.
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      if (cancelled) return;
      try {
        const size = 64;
        const canvas = document.createElement("canvas");
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext("2d");
        if (!ctx) throw new Error("sin contexto 2d");
        const margin = 4;
        const maxDim = size - margin * 2;
        const scale = Math.min(maxDim / img.width, maxDim / img.height);
        const w = img.width * scale;
        const h = img.height * scale;
        ctx.drawImage(img, (size - w) / 2, (size - h) / 2, w, h);
        setFavicon(canvas.toDataURL("image/png"));
      } catch {
        // Host del logo sin CORS habilitado - el lienzo queda "tainted" y
        // toDataURL lanza. Se usa la URL cruda tal cual funcionaba antes.
        setFavicon(tenant.logo_url!);
      }
    };
    img.onerror = () => setFavicon(tenant.logo_url!);
    img.src = tenant.logo_url;

    return () => {
      cancelled = true;
    };
  }, [tenant?.logo_url]);

  // Backend (services/core role_guard) is the real enforcement — this is only
  // to stop a Cajero/Almacén/Contador from landing on a section they can't
  // use if they type the URL directly, since the sidebar already hides it.
  useEffect(() => {
    if (usuario?.rol && !isRouteAllowed(pathname, usuario.rol)) {
      router.replace("/dashboard?denied=1");
      return;
    }
    if (tenant && tenant.factura_electronica_activa === false && isDgiiRoute(pathname)) {
      router.replace("/dashboard?denied=dgii");
      return;
    }
    if (!isModuloAllowed(pathname, modulosActivos)) {
      router.replace("/dashboard");
    }
  }, [pathname, usuario, tenant, modulosActivos, licencia, router]);

  const navFiltered = NAV.map((group) => ({
    ...group,
    items: group.items.filter(
      (item) =>
        isRouteAllowed(item.href, usuario?.rol) &&
        !(tenant?.factura_electronica_activa === false && isDgiiRoute(item.href)) &&
        isModuloAllowed(item.href, modulosActivos)
    ),
  })).filter((group) => group.items.length > 0);

  function cerrarSesion() {
    localStorage.removeItem("token");
    localStorage.removeItem("usuario");
    localStorage.removeItem("tenant");
    document.cookie = "token=; path=/; max-age=0";
    router.push("/login");
  }

  const iniciales = (usuario?.nombre || "U")
    .split(" ")
    .map((p) => p[0])
    .slice(0, 2)
    .join("")
    .toUpperCase();

  return (
    <div className="h-screen overflow-hidden bg-background">
      <div className="flex h-full">
        <aside className="hidden lg:flex lg:flex-col w-[248px] shrink-0 border-r border-border bg-surface h-full overflow-y-auto">
          <Link href="/dashboard" className="flex items-center gap-2.5 px-5 h-16 border-b border-border shrink-0">
            {tenant?.logo_url ? (
              <img src={tenant.logo_url} alt="" className="h-8 w-8 rounded-md object-cover shrink-0 border border-border" />
            ) : (
              <div className="h-8 w-8 rounded-md bg-primary text-primary-foreground flex items-center justify-center shadow-xs shrink-0">
                <Store className="h-4 w-4" />
              </div>
            )}
            <div className="min-w-0">
              <p className="font-serif font-semibold text-[15px] truncate leading-tight">
                {tenant?.nombre_comercial || tenant?.razon_social || "Colmado POS"}
              </p>
              <p className="text-[10px] text-muted-foreground/70 leading-none">by Colmado POS</p>
            </div>
          </Link>
          <nav className="flex-1 overflow-y-auto py-4 px-3 space-y-3">
            {navFiltered.map((group) => {
              const hasActiveItem = group.items.some((item) => pathname === item.href);
              const expanded = hasActiveItem || !collapsedGroups.has(group.title);
              return (
                <div key={group.title}>
                  <button
                    type="button"
                    onClick={() => toggleGroup(group.title)}
                    className="w-full flex items-center justify-between px-2.5 mb-1.5 text-[10.5px] font-bold uppercase tracking-wide text-muted-foreground/70 hover:text-foreground/80 transition-colors"
                  >
                    {group.title}
                    {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
                  </button>
                  {expanded && (
                    <div className="space-y-0.5">
                      {group.items.map((item) => {
                        const active = pathname === item.href;
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
                    </div>
                  )}
                </div>
              );
            })}
          </nav>
          <div className="border-t border-border p-3">
            <div className="flex items-center gap-2.5 px-2 py-2 rounded-md">
              <div className="h-8 w-8 rounded-full bg-primary/10 text-primary flex items-center justify-center text-xs font-bold shrink-0">
                {iniciales}
              </div>
              <div className="min-w-0">
                <p className="text-sm font-medium truncate">{usuario?.nombre || "Usuario"}</p>
                <p className="text-xs text-muted-foreground truncate">{usuario?.rol || tenant?.razon_social || ""}</p>
              </div>
            </div>
          </div>
        </aside>

        <div className="flex-1 min-w-0 h-full overflow-y-auto">
          <header className="h-16 border-b border-border bg-surface/90 backdrop-blur-sm sticky top-0 z-20 flex items-center justify-between px-5">
            <div className="lg:hidden flex items-center gap-2 min-w-0">
              {tenant?.logo_url ? (
                <img src={tenant.logo_url} alt="" className="h-7 w-7 rounded-md object-cover shrink-0 border border-border" />
              ) : (
                <div className="h-7 w-7 rounded-md bg-primary text-primary-foreground flex items-center justify-center shrink-0">
                  <Store className="h-3.5 w-3.5" />
                </div>
              )}
              <span className="font-serif font-semibold text-sm truncate">{tenant?.nombre_comercial || tenant?.razon_social || "Colmado POS"}</span>
            </div>
            <div className="hidden lg:block" />
            <button
              onClick={cerrarSesion}
              className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              <LogOut className="h-4 w-4" />
              Cerrar sesión
            </button>
          </header>
          {licencia?.status === "trial" && licencia.dias_restantes <= 14 && (
            <div className="flex items-center gap-2 border-b border-warning/20 bg-warning/10 text-warning px-5 py-2 text-sm">
              <AlertTriangle className="h-4 w-4 shrink-0" />
              <span>
                {licencia.dias_restantes <= 0
                  ? "Tu período de prueba termina hoy."
                  : `Tu período de prueba termina en ${licencia.dias_restantes} ${licencia.dias_restantes === 1 ? "día" : "días"}.`}{" "}
                Contáctanos para activar tu licencia.
              </span>
            </div>
          )}
          <main className="p-6">{children}</main>
        </div>
      </div>
      {isModuloActivo("IA_ASISTENTE", modulosActivos) && <AiChatWidget />}
    </div>
  );
}
