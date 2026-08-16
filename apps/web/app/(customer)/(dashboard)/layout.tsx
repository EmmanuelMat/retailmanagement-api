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
  Compass,
  X,
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
    title: "Compras",
    items: [
      { label: "Compras", href: "/compras", icon: ShoppingBag },
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

// Recorrido guiado: apunta a los mismos href que ya existen en NAV, así
// que no depende de nada más que el sidebar real - si el usuario no tiene
// permiso para ver un ítem, ese paso simplemente se filtra del recorrido.
const TOUR_STEPS: { href: string; title: string; body: string }[] = [
  { href: "/dashboard", title: "Tu resumen del día", body: "Ventas, caja e inventario de un vistazo cada vez que entras." },
  { href: "/pos", title: "Punto de Venta", body: "Cobra y el e-CF se firma y numera solo, sin pasos de facturación aparte." },
  { href: "/conduces", title: "Conduces", body: "Entregas de mercancía — incluso de forma retroactiva si el cajero olvidó marcarla, con permiso." },
  { href: "/inventario/productos", title: "Inventario", body: "El stock se descuenta y repone solo con cada venta y compra." },
  { href: "/compras", title: "Compras", body: "Cada compra a suplidor queda lista para el reporte 606 del mes." },
  { href: "/contabilidad", title: "Contabilidad", body: "Libro Diario y Mayor se arman solos — sin asientos manuales." },
  { href: "/nomina", title: "Nómina", body: "Empleados, adelantos, y el cálculo exacto de neto con Mandato." },
  { href: "/reportes/dgii", title: "Reportes DGII", body: "606 e IT-1 listos para declarar, sin reconstruir nada a mano." },
  { href: "/configuracion/usuarios", title: "Usuarios y Roles", body: "Arma los permisos a la medida y restablece contraseñas sin correo." },
];

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const [tenant, setTenant] = useState<{ razon_social?: string; nombre_comercial?: string | null; logo_url?: string | null; factura_electronica_activa?: boolean } | null>(null);
  const [usuario, setUsuario] = useState<{ nombre?: string; rol?: string; es_admin?: boolean; permisos?: string[]; must_change_password?: boolean } | null>(null);
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

  const [tourOpen, setTourOpen] = useState(false);
  const [tourIndex, setTourIndex] = useState(0);

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
    if (usuario?.must_change_password) {
      router.replace("/cambiar-password");
      return;
    }
    if (usuario && !isRouteAllowed(pathname, usuario)) {
      router.replace("/dashboard?denied=1");
      return;
    }
    if (tenant && tenant.factura_electronica_activa === false && isDgiiRoute(pathname)) {
      router.replace("/dashboard?denied=dgii");
      return;
    }
    if (!isModuloAllowed(pathname, modulosActivos, licencia?.status)) {
      router.replace("/dashboard");
    }
  }, [pathname, usuario, tenant, modulosActivos, licencia, router]);

  const navFiltered = NAV.map((group) => ({
    ...group,
    items: group.items.filter(
      (item) =>
        isRouteAllowed(item.href, usuario) &&
        !(tenant?.factura_electronica_activa === false && isDgiiRoute(item.href)) &&
        isModuloAllowed(item.href, modulosActivos, licencia?.status)
    ),
  })).filter((group) => group.items.length > 0);

  const visibleHrefs = new Set(navFiltered.flatMap((g) => g.items.map((i) => i.href)));
  const tourSteps = TOUR_STEPS.filter((s) => visibleHrefs.has(s.href));

  // Si el paso activo vive dentro de un grupo colapsado, lo expande - el
  // recorrido no debería depender de que el usuario ya haya abierto esa
  // sección del sidebar.
  useEffect(() => {
    if (!tourOpen) return;
    const step = tourSteps[tourIndex];
    if (!step) return;
    const group = NAV.find((g) => g.items.some((i) => i.href === step.href));
    if (!group) return;
    setCollapsedGroups((prev) => {
      if (!prev.has(group.title)) return prev;
      const next = new Set(prev);
      next.delete(group.title);
      try {
        localStorage.setItem("nav_collapsed_groups", JSON.stringify([...next]));
      } catch {}
      return next;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tourOpen, tourIndex]);

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
            <div className="flex items-center gap-4">
              {tourSteps.length > 0 && (
                <button
                  onClick={() => {
                    setTourIndex(0);
                    setTourOpen(true);
                  }}
                  className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
                >
                  <Compass className="h-4 w-4" />
                  Recorrido
                </button>
              )}
              <button
                onClick={cerrarSesion}
                className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
                <LogOut className="h-4 w-4" />
                Cerrar sesión
              </button>
            </div>
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
      {isModuloActivo("IA_ASISTENTE", modulosActivos, licencia?.status) && <AiChatWidget />}
      {tourOpen && tourSteps.length > 0 && (
        <ProductTour
          steps={tourSteps}
          index={Math.min(tourIndex, tourSteps.length - 1)}
          onNext={() => setTourIndex((i) => Math.min(i + 1, tourSteps.length - 1))}
          onPrev={() => setTourIndex((i) => Math.max(i - 1, 0))}
          onClose={() => setTourOpen(false)}
          onFinish={() => setTourOpen(false)}
        />
      )}
    </div>
  );
}

function ProductTour({
  steps,
  index,
  onNext,
  onPrev,
  onClose,
  onFinish,
}: {
  steps: { href: string; title: string; body: string }[];
  index: number;
  onNext: () => void;
  onPrev: () => void;
  onClose: () => void;
  onFinish: () => void;
}) {
  const [rect, setRect] = useState<DOMRect | null>(null);
  const step = steps[index];

  useEffect(() => {
    if (!step) return;
    let cancelled = false;
    let raf1 = 0;
    let raf2 = 0;
    let attempts = 0;

    // El paso puede apuntar a un ítem dentro de un grupo que el layout
    // todavía está expandiendo (efecto aparte, en el componente padre) -
    // reintenta unos cuantos frames en vez de rendirse al primer intento.
    function tryMeasure() {
      if (cancelled) return;
      const el = document.querySelector<HTMLAnchorElement>(`aside nav a[href="${CSS.escape(step.href)}"]`);
      if (!el) {
        if (attempts++ < 30) {
          raf1 = requestAnimationFrame(tryMeasure);
        } else {
          setRect(null);
        }
        return;
      }
      el.scrollIntoView({ block: "nearest" });
      raf1 = requestAnimationFrame(() => {
        raf2 = requestAnimationFrame(() => {
          if (!cancelled) setRect(el.getBoundingClientRect());
        });
      });
    }

    tryMeasure();
    window.addEventListener("resize", tryMeasure);
    window.addEventListener("scroll", tryMeasure, true);
    return () => {
      cancelled = true;
      cancelAnimationFrame(raf1);
      cancelAnimationFrame(raf2);
      window.removeEventListener("resize", tryMeasure);
      window.removeEventListener("scroll", tryMeasure, true);
    };
  }, [step]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowRight") onNext();
      if (e.key === "ArrowLeft") onPrev();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, onNext, onPrev]);

  if (!step) return null;

  const cardWidth = 320;
  const top = rect
    ? Math.min(Math.max(rect.top, 16), window.innerHeight - 240)
    : window.innerHeight / 2 - 110;
  const left = rect
    ? Math.min(rect.right + 16, window.innerWidth - cardWidth - 16)
    : window.innerWidth / 2 - cardWidth / 2;

  return (
    <div className="fixed inset-0 z-[100]">
      {rect ? (
        <div
          className="fixed rounded-md pointer-events-none transition-all duration-200"
          style={{
            top: rect.top - 4,
            left: rect.left - 4,
            width: rect.width + 8,
            height: rect.height + 8,
            boxShadow: "0 0 0 9999px hsl(var(--foreground) / 0.55)",
            outline: "2px solid hsl(var(--accent))",
          }}
        />
      ) : (
        <div className="fixed inset-0 bg-foreground/55" />
      )}
      <div
        className="fixed rounded-lg border border-border bg-surface-raised shadow-lg p-5 transition-all duration-200"
        style={{ top, left, width: cardWidth }}
      >
        <div className="flex items-center justify-between">
          <span className="font-mono text-xs text-muted-foreground">
            {index + 1} / {steps.length}
          </span>
          <button onClick={onClose} aria-label="Cerrar recorrido" className="text-muted-foreground hover:text-foreground">
            <X className="h-4 w-4" />
          </button>
        </div>
        <h3 className="mt-2 font-serif font-semibold text-base">{step.title}</h3>
        <p className="mt-1.5 text-sm text-foreground-soft leading-relaxed">{step.body}</p>
        <div className="mt-4 flex items-center justify-between gap-2">
          <button onClick={onClose} className="text-xs text-muted-foreground hover:text-foreground">
            Saltar
          </button>
          <div className="flex items-center gap-2">
            {index > 0 && (
              <button onClick={onPrev} className="h-8 px-3 rounded-md border border-border text-sm hover:bg-muted">
                Atrás
              </button>
            )}
            <button
              onClick={index === steps.length - 1 ? onFinish : onNext}
              className="h-8 px-3 rounded-md bg-primary text-primary-foreground text-sm font-medium hover:bg-primary-hover"
            >
              {index === steps.length - 1 ? "Listo" : "Siguiente"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
