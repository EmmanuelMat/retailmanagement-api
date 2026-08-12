"use client";

import { useState } from "react";
import Link from "next/link";
import {
  Store,
  Menu,
  X,
  ArrowRight,
  ShieldCheck,
  Package,
  BookOpen,
  HandCoins,
  FileBarChart,
  Check,
  ScanLine,
  Landmark,
  Lock,
} from "lucide-react";
import { Button, Badge, Card } from "@repo/ui";

const NAV_LINKS = [
  { label: "Producto", href: "#producto" },
  { label: "Funciones", href: "#funciones" },
  { label: "Precios", href: "#precios" },
  { label: "Preguntas", href: "#preguntas" },
];

const FEATURES = [
  {
    icon: ScanLine,
    title: "Punto de venta al instante",
    body: "Cobra en segundos, genera el e-CF firmado y entrega el QR de la DGII sin que el cliente espere en fila.",
  },
  {
    icon: Package,
    title: "Inventario en tiempo real",
    body: "Cada venta descuenta el inventario al momento. Alertas de stock mínimo antes de que se te acabe el producto.",
  },
  {
    icon: BookOpen,
    title: "Contabilidad de partida doble",
    body: "Ventas, compras, gastos, nómina y adelantos generan su asiento contable con un clic. Libro mayor y balance de comprobación siempre cuadrados.",
  },
  {
    icon: HandCoins,
    title: "Nómina con adelantos del 50%",
    body: "Tus empleados piden adelanto de lo que ya ganaron, tú apruebas desde el celular, la nómina descuenta sola.",
  },
  {
    icon: FileBarChart,
    title: "Reportes DGII listos",
    body: "606, 608, IT-1 generados automáticamente. Súbelos y ya — sin hojas de cálculo a última hora.",
  },
  {
    icon: ShieldCheck,
    title: "Cuadre de caja sin sorpresas",
    body: "Abre y cierra turno, cuenta el efectivo, y el sistema calcula la diferencia al instante. Sabes exactamente cuánto entró y cuánto salió.",
  },
];

const PLANS = [
  {
    name: "Colmado",
    price: "1,490",
    period: "/mes",
    description: "Para un negocio con un solo punto de venta.",
    cta: "Empieza gratis",
    features: [
      "1 terminal POS",
      "Facturación e-CF ilimitada",
      "Inventario y clientes",
      "Reportes DGII 606",
      "Soporte por WhatsApp",
    ],
    highlighted: false,
  },
  {
    name: "Negocio",
    price: "2,990",
    period: "/mes",
    description: "Para negocios que ya llevan contabilidad y nómina.",
    cta: "Empieza gratis",
    features: [
      "Hasta 5 terminales POS",
      "Todo lo de Colmado",
      "Contabilidad y libro mayor",
      "Nómina y adelantos 50%",
      "Bancos y conciliación",
      "Soporte prioritario",
    ],
    highlighted: true,
  },
  {
    name: "Cadena",
    price: "A medida",
    period: "",
    description: "Para varios locales bajo un mismo RNC.",
    cta: "Hablar con ventas",
    features: [
      "Terminales ilimitadas",
      "Todo lo de Negocio",
      "Multi-sucursal consolidado",
      "Acceso a API y webhooks",
      "Onboarding asistido",
    ],
    highlighted: false,
  },
];

const FAQS = [
  {
    q: "¿Esto cumple con la Ley 32-23 de facturación electrónica?",
    a: "Sí. Firmamos cada comprobante con XAdES-BES real (C14N, RSA-SHA256) siguiendo el estándar de la DGII, y generamos el QR de verificación en el momento del cobro. Los reportes 606, 608 e IT-1 salen del mismo dato, así que nunca hay descuadre.",
  },
  {
    q: "¿Qué pasa si se va la internet en el colmado?",
    a: "El terminal POS sigue vendiendo localmente y sincroniza los comprobantes en cuanto vuelve la conexión. Nada se pierde: cada evento queda encadenado con hash antes de subir.",
  },
  {
    q: "¿Cómo funciona el adelanto de sueldo del 50%?",
    a: "No es un préstamo — es sueldo que el empleado ya ganó. El sistema calcula el 50% disponible en tiempo real, el empleado lo pide desde el celular, tú apruebas, y se descuenta solo en la próxima nómina.",
  },
  {
    q: "¿Necesito instalar algo o comprar hardware especial?",
    a: "No. Corre en cualquier navegador, tablet o la impresora térmica que ya tengas. La app móvil es opcional para que tus empleados vean su disponible de adelanto.",
  },
  {
    q: "¿Puedo migrar mi inventario y clientes actuales?",
    a: "Sí, importamos tu catálogo desde Excel o desde tu sistema actual en el proceso de onboarding, sin costo adicional.",
  },
];

function MobileNav({ open, onClose }: { open: boolean; onClose: () => void }) {
  if (!open) return null;
  return (
    <div className="lg:hidden fixed inset-0 z-50 bg-background">
      <div className="flex items-center justify-between h-16 px-5 border-b border-border">
        <BrandMark />
        <button onClick={onClose} aria-label="Cerrar menú" className="h-9 w-9 flex items-center justify-center rounded-md hover:bg-muted">
          <X className="h-5 w-5" />
        </button>
      </div>
      <nav className="flex flex-col gap-1 p-5">
        {NAV_LINKS.map((link) => (
          <a key={link.href} href={link.href} onClick={onClose} className="px-3 py-3 rounded-md text-base font-medium hover:bg-muted">
            {link.label}
          </a>
        ))}
        <div className="h-px bg-border my-3" />
        <Link href="/login" onClick={onClose} className="px-3 py-3 rounded-md text-base font-medium hover:bg-muted">
          Iniciar sesión
        </Link>
        <Link href="/registro" onClick={onClose}>
          <Button className="w-full mt-2" size="lg">
            Empieza gratis
          </Button>
        </Link>
      </nav>
    </div>
  );
}

function BrandMark() {
  return (
    <Link href="/" className="flex items-center gap-2.5">
      <div className="h-9 w-9 rounded-md bg-primary text-primary-foreground flex items-center justify-center shadow-xs">
        <Store className="h-4.5 w-4.5" />
      </div>
      <span className="font-serif font-semibold text-[17px] leading-none">Colmado POS</span>
    </Link>
  );
}

export default function LandingPage() {
  const [menuOpen, setMenuOpen] = useState(false);

  return (
    <div className="min-h-screen bg-background">
      <MobileNav open={menuOpen} onClose={() => setMenuOpen(false)} />

      {/* Nav */}
      <header className="sticky top-0 z-40 border-b border-border bg-background/80 backdrop-blur-md">
        <div className="mx-auto max-w-6xl h-16 px-5 flex items-center justify-between">
          <BrandMark />
          <nav className="hidden lg:flex items-center gap-1">
            {NAV_LINKS.map((link) => (
              <a
                key={link.href}
                href={link.href}
                className="px-3.5 py-2 rounded-md text-sm font-medium text-foreground-soft hover:text-foreground hover:bg-muted transition-colors"
              >
                {link.label}
              </a>
            ))}
          </nav>
          <div className="hidden lg:flex items-center gap-2">
            <Link href="/login">
              <Button variant="ghost">Iniciar sesión</Button>
            </Link>
            <Link href="/registro">
              <Button>
                Empieza gratis
                <ArrowRight className="h-4 w-4" />
              </Button>
            </Link>
          </div>
          <button
            onClick={() => setMenuOpen(true)}
            aria-label="Abrir menú"
            className="lg:hidden h-9 w-9 flex items-center justify-center rounded-md hover:bg-muted"
          >
            <Menu className="h-5 w-5" />
          </button>
        </div>
      </header>

      {/* Hero */}
      <section className="relative overflow-hidden bg-hero-glow">
        <div className="absolute inset-0 bg-grid-dots [mask-image:radial-gradient(ellipse_60%_50%_at_50%_0%,black,transparent)]" />
        <div className="relative mx-auto max-w-6xl px-5 pt-16 pb-20 lg:pt-24 lg:pb-28">
          <div className="max-w-3xl animate-fade-up">
            <Badge variant="accent" className="mb-5">
              <ShieldCheck className="h-3.5 w-3.5 mr-1.5" />
              Listo para la Ley 32-23 DGII — obligatoria desde nov. 2026
            </Badge>
            <h1 className="text-[2.5rem] leading-[1.08] sm:text-6xl sm:leading-[1.05] font-bold tracking-tight">
              El punto de venta completo para tu colmado
            </h1>
            <p className="mt-6 text-lg sm:text-xl text-foreground-soft max-w-2xl leading-relaxed">
              Cobra, factura e-CF ante la DGII, controla inventario, lleva la contabilidad y paga adelantos a tus
              empleados — todo desde un solo lugar, en español dominicano.
            </p>
            <div className="mt-9 flex flex-col sm:flex-row gap-3">
              <Link href="/registro">
                <Button size="xl" className="w-full sm:w-auto">
                  Empieza gratis
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </Link>
              <a href="#producto">
                <Button size="xl" variant="outline" className="w-full sm:w-auto">
                  Ver cómo funciona
                </Button>
              </a>
            </div>
            <div className="mt-8 flex flex-wrap items-center gap-x-6 gap-y-2 text-sm text-muted-foreground">
              <span className="flex items-center gap-1.5"><Check className="h-4 w-4 text-success" /> Sin tarjeta de crédito</span>
              <span className="flex items-center gap-1.5"><Check className="h-4 w-4 text-success" /> Configurado en un día</span>
              <span className="flex items-center gap-1.5"><Check className="h-4 w-4 text-success" /> Soporte en español</span>
            </div>
          </div>

          {/* Product mockup */}
          <div className="mt-16 animate-fade-up [animation-delay:120ms]">
            <ProductMockup />
          </div>
        </div>
      </section>

      {/* Trust bar */}
      <section className="border-y border-border bg-muted/30">
        <div className="mx-auto max-w-6xl px-5 py-6 flex flex-wrap items-center justify-center gap-x-10 gap-y-3 text-sm font-medium text-muted-foreground">
          <span className="flex items-center gap-2"><ShieldCheck className="h-4 w-4" /> Firma XAdES-BES real</span>
          <span className="flex items-center gap-2"><Landmark className="h-4 w-4" /> Contabilidad de doble entrada</span>
          <span className="flex items-center gap-2"><Lock className="h-4 w-4" /> Eventos encadenados con hash</span>
          <span className="flex items-center gap-2"><FileBarChart className="h-4 w-4" /> Reportes 606 · 608 · IT-1</span>
        </div>
      </section>

      {/* Features */}
      <section id="funciones" className="mx-auto max-w-6xl px-5 py-20 lg:py-28">
        <div className="max-w-2xl">
          <p className="text-sm font-bold uppercase tracking-wide text-primary">Todo en un solo sistema</p>
          <h2 className="mt-3 text-3xl sm:text-4xl font-bold tracking-tight">
            Lo que antes eran cinco herramientas sueltas
          </h2>
          <p className="mt-4 text-lg text-foreground-soft">
            Colmado POS reemplaza la libreta, la calculadora, el Excel de inventario y la contabilidad hecha a mano
            por un solo sistema que habla entre sí.
          </p>
        </div>
        <div className="mt-12 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
          {FEATURES.map((f) => (
            <Card key={f.title} className="p-6 hover:shadow-card hover:-translate-y-0.5 transition-all">
              <div className="h-10 w-10 rounded-md bg-primary/10 text-primary flex items-center justify-center">
                <f.icon className="h-5 w-5" />
              </div>
              <h3 className="mt-4 font-semibold text-base">{f.title}</h3>
              <p className="mt-2 text-sm text-foreground-soft leading-relaxed">{f.body}</p>
            </Card>
          ))}
        </div>
      </section>

      {/* Product showcase */}
      <section id="producto" className="bg-muted/30 border-y border-border">
        <div className="mx-auto max-w-6xl px-5 py-20 lg:py-28 space-y-24">
          <ShowcaseRow
            eyebrow="Punto de venta"
            title="Cobra en segundos, factura sin pensarlo"
            body="El cajero busca el producto, cobra, y el sistema genera y firma el e-CF automáticamente — con ITBIS desglosado y QR de la DGII listo para imprimir o enviar."
            bullets={["ITBIS 18%, 16% y exento calculado solo", "Código de seguridad y QR en cada recibo", "Funciona con la impresora térmica que ya tienes"]}
            icon={ScanLine}
          />
          <ShowcaseRow
            eyebrow="Nómina y adelantos"
            title="Adelantos de sueldo sin que sea un préstamo"
            body="Tus empleados ven cuánto han ganado hoy y cuánto tienen disponible (50%). Piden el adelanto, tú apruebas desde el celular, y se descuenta solo en la próxima quincena."
            bullets={["0% de interés — no es deuda, es sueldo ganado", "Aprobación en un toque desde el celular", "Contabilidad del adelanto generada sola"]}
            icon={HandCoins}
            reverse
          />
          <ShowcaseRow
            eyebrow="Contabilidad"
            title="El libro mayor se arma con un clic"
            body="Ventas, compras, gastos, nómina y adelantos generan sus asientos de partida doble — sincroniza cuando quieras y el libro mayor queda al día. Cierre de mes sin sorpresas."
            bullets={["Partida doble para cada venta, compra, gasto y adelanto", "Libro mayor y balance siempre cuadrados", "Cierre de períodos con un clic"]}
            icon={BookOpen}
          />
        </div>
      </section>

      {/* Pricing */}
      <section id="precios" className="mx-auto max-w-6xl px-5 py-20 lg:py-28">
        <div className="text-center max-w-2xl mx-auto">
          <p className="text-sm font-bold uppercase tracking-wide text-primary">Precios</p>
          <h2 className="mt-3 text-3xl sm:text-4xl font-bold tracking-tight">Un plan para cada tamaño de negocio</h2>
          <p className="mt-4 text-lg text-foreground-soft">Precios en pesos dominicanos. Cambia de plan cuando quieras.</p>
        </div>
        <div className="mt-12 grid grid-cols-1 lg:grid-cols-3 gap-6 items-start">
          {PLANS.map((plan) => (
            <Card
              key={plan.name}
              className={
                plan.highlighted
                  ? "p-8 border-primary shadow-glow relative lg:-translate-y-3"
                  : "p-8"
              }
            >
              {plan.highlighted && (
                <Badge variant="accent" className="absolute -top-3 left-8">
                  Más popular
                </Badge>
              )}
              <h3 className="font-semibold text-lg">{plan.name}</h3>
              <p className="mt-1.5 text-sm text-muted-foreground">{plan.description}</p>
              <div className="mt-5 flex items-baseline gap-1">
                {plan.price !== "A medida" && <span className="text-lg font-medium text-muted-foreground">RD$</span>}
                <span className="text-4xl font-bold tracking-tight">{plan.price}</span>
                <span className="text-sm text-muted-foreground">{plan.period}</span>
              </div>
              <Link href="/registro" className="block mt-6">
                <Button className="w-full" size="lg" variant={plan.highlighted ? "default" : "outline"}>
                  {plan.cta}
                </Button>
              </Link>
              <ul className="mt-7 space-y-3">
                {plan.features.map((f) => (
                  <li key={f} className="flex items-start gap-2.5 text-sm">
                    <Check className="h-4 w-4 text-success mt-0.5 shrink-0" />
                    <span className="text-foreground-soft">{f}</span>
                  </li>
                ))}
              </ul>
            </Card>
          ))}
        </div>
      </section>

      {/* FAQ */}
      <section id="preguntas" className="bg-muted/30 border-y border-border">
        <div className="mx-auto max-w-3xl px-5 py-20 lg:py-28">
          <div className="text-center">
            <p className="text-sm font-bold uppercase tracking-wide text-primary">Preguntas frecuentes</p>
            <h2 className="mt-3 text-3xl sm:text-4xl font-bold tracking-tight">Todo lo que quieres saber</h2>
          </div>
          <div className="mt-12 space-y-3">
            {FAQS.map((item) => (
              <details key={item.q} className="group rounded-lg border border-border bg-surface open:shadow-xs">
                <summary className="cursor-pointer list-none flex items-center justify-between gap-4 px-5 py-4 font-medium text-[15px]">
                  {item.q}
                  <span className="shrink-0 text-muted-foreground transition-transform group-open:rotate-45 text-xl leading-none">+</span>
                </summary>
                <p className="px-5 pb-5 text-sm text-foreground-soft leading-relaxed">{item.a}</p>
              </details>
            ))}
          </div>
        </div>
      </section>

      {/* Final CTA */}
      <section className="mx-auto max-w-6xl px-5 py-20 lg:py-28">
        <div className="relative overflow-hidden rounded-2xl bg-primary text-primary-foreground px-8 py-16 text-center shadow-lifted">
          <div className="absolute inset-0 bg-grid-dots opacity-10 [mask-image:radial-gradient(ellipse_60%_60%_at_50%_50%,black,transparent)]" />
          <div className="relative">
            <h2 className="text-3xl sm:text-4xl font-bold tracking-tight">La Ley 32-23 es obligatoria en noviembre 2026</h2>
            <p className="mt-4 text-lg text-primary-foreground/85 max-w-xl mx-auto">
              Empieza hoy y llega listo — sin correr contra el reloj a última hora.
            </p>
            <div className="mt-8 flex flex-col sm:flex-row gap-3 justify-center">
              <Link href="/registro">
                <Button size="xl" variant="accent" className="w-full sm:w-auto">
                  Empieza gratis ahora
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </Link>
              <Link href="/login">
                <Button size="xl" variant="outline" className="w-full sm:w-auto bg-transparent border-primary-foreground/30 text-primary-foreground hover:bg-primary-foreground/10">
                  Iniciar sesión
                </Button>
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-border">
        <div className="mx-auto max-w-6xl px-5 py-12 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-6">
          <BrandMark />
          <nav className="flex flex-wrap gap-x-6 gap-y-2 text-sm text-muted-foreground">
            {NAV_LINKS.map((link) => (
              <a key={link.href} href={link.href} className="hover:text-foreground transition-colors">
                {link.label}
              </a>
            ))}
            <Link href="/login" className="hover:text-foreground transition-colors">Iniciar sesión</Link>
          </nav>
        </div>
        <div className="mx-auto max-w-6xl px-5 pb-10 text-xs text-muted-foreground">
          Hecho en 🇩🇴 Santo Domingo. Colmado POS © {new Date().getFullYear()}.
        </div>
      </footer>
    </div>
  );
}

function ShowcaseRow({
  eyebrow,
  title,
  body,
  bullets,
  icon: Icon,
  reverse,
}: {
  eyebrow: string;
  title: string;
  body: string;
  bullets: string[];
  icon: React.ComponentType<{ className?: string }>;
  reverse?: boolean;
}) {
  return (
    <div className={`grid grid-cols-1 lg:grid-cols-2 gap-10 lg:gap-16 items-center ${reverse ? "lg:[&>*:first-child]:order-2" : ""}`}>
      <div>
        <p className="text-sm font-bold uppercase tracking-wide text-primary">{eyebrow}</p>
        <h3 className="mt-3 text-2xl sm:text-3xl font-bold tracking-tight">{title}</h3>
        <p className="mt-4 text-foreground-soft leading-relaxed">{body}</p>
        <ul className="mt-6 space-y-3">
          {bullets.map((b) => (
            <li key={b} className="flex items-start gap-2.5 text-sm">
              <Check className="h-4 w-4 text-success mt-0.5 shrink-0" />
              <span>{b}</span>
            </li>
          ))}
        </ul>
      </div>
      <Card className="p-10 flex items-center justify-center bg-surface">
        <div className="h-16 w-16 rounded-xl bg-primary/10 text-primary flex items-center justify-center">
          <Icon className="h-8 w-8" />
        </div>
      </Card>
    </div>
  );
}

function ProductMockup() {
  return (
    <div className="rounded-xl border border-border bg-surface shadow-lifted overflow-hidden max-w-4xl mx-auto">
      <div className="h-9 bg-muted/60 border-b border-border flex items-center gap-1.5 px-4">
        <span className="h-2.5 w-2.5 rounded-full bg-destructive/50" />
        <span className="h-2.5 w-2.5 rounded-full bg-warning/50" />
        <span className="h-2.5 w-2.5 rounded-full bg-success/50" />
      </div>
      <div className="grid grid-cols-[180px_1fr]">
        <div className="hidden sm:block border-r border-border bg-muted/30 p-4 space-y-2">
          <div className="h-7 w-7 rounded-md bg-primary/20" />
          {["Dashboard", "Punto de Venta", "Inventario", "Contabilidad", "Nómina"].map((label, i) => (
            <div key={label} className={`h-8 rounded-md flex items-center px-2.5 text-xs font-medium ${i === 1 ? "bg-primary/10 text-primary" : "text-muted-foreground"}`}>
              {label}
            </div>
          ))}
        </div>
        <div className="p-5 space-y-4">
          <div className="flex items-center justify-between">
            <div className="h-4 w-40 rounded bg-muted" />
            <div className="h-8 w-28 rounded-md bg-primary" />
          </div>
          <div className="grid grid-cols-3 gap-3">
            {["RD$8,420", "RD$1,516", "42 uds"].map((v, i) => (
              <div key={i} className="rounded-lg border border-border p-3">
                <div className="h-2.5 w-16 rounded bg-muted mb-2" />
                <div className="text-base font-bold">{v}</div>
              </div>
            ))}
          </div>
          <div className="rounded-lg border border-border overflow-hidden">
            <div className="h-8 bg-muted/50 border-b border-border" />
            {[1, 2, 3].map((r) => (
              <div key={r} className="h-9 border-b border-border last:border-0 flex items-center px-3 gap-3">
                <div className="h-2.5 w-2.5 rounded-full bg-success" />
                <div className="h-2.5 w-32 rounded bg-muted" />
                <div className="h-2.5 w-16 rounded bg-muted ml-auto" />
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
