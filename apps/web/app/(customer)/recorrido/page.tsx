"use client";

import Link from "next/link";
import { ArrowLeft, Store, Printer } from "lucide-react";
import { Badge, Button, Card } from "@repo/ui";

const RECORRIDO = [
  {
    code: "PV",
    title: "Punto de Venta",
    subtitle: "Ventas · Cotizaciones · Conduces · Clientes",
    paragraphs: [
      "El cajero cobra desde una sola pantalla: busca el producto, cobra en efectivo, tarjeta o mixto, y el sistema emite el e-CF firmado y numerado automáticamente — sin pasos manuales de facturación.",
      "Una cotización se convierte en venta con un clic cuando el cliente confirma. Y si el cajero olvida marcar la entrega diferida al momento de vender, el conduce se puede generar después, con permiso, sin tocar el inventario ya descontado.",
    ],
    steps: [
      "Abrir Punto de Venta, agregar productos y cobrar.",
      "Convertir una cotización guardada en venta.",
      "Emitir el conduce al momento — o de forma retroactiva si se olvidó.",
    ],
    badge: "e-CF",
    badgeVariant: "success" as const,
    calloutTitle: "Firmado al vuelo",
    calloutBody: "Cada venta genera su comprobante fiscal electrónico automáticamente — el cajero nunca lo piensa.",
  },
  {
    code: "IV",
    title: "Inventario y Compras",
    subtitle: "Productos · Categorías · Movimientos · Proveedores",
    paragraphs: [
      "El catálogo se organiza por categorías, y el stock se mueve solo: cada venta lo descuenta, cada compra lo repone. Nadie ajusta números a mano.",
      "Las compras quedan registradas por suplidor con ITBIS, retenciones y tipo de bien o servicio — listas para el reporte 606, no como un dato aparte que hay que reconstruir a fin de mes.",
    ],
    steps: [
      "Registrar el producto, su categoría y precio.",
      "Cada venta y compra ajusta el stock automáticamente.",
      "Cada compra a suplidor queda lista para el 606 del mes.",
    ],
    badge: "STK",
    badgeVariant: "default" as const,
    calloutTitle: "Stock sin ajustes manuales",
    calloutBody: "El módulo de Movimientos muestra de dónde salió o entró cada unidad, sin hojas de cálculo aparte.",
  },
  {
    code: "CT",
    title: "Contabilidad, Caja y Bancos",
    subtitle: "Libro Diario · Libro Mayor · Caja · Bancos",
    paragraphs: [
      "No hay asientos manuales que llevar: cada venta y cada compra se contabiliza sola, sobre un catálogo de cuentas real — Libro Diario y Libro Mayor listos para revisar cuando quieras, no solo a fin de año.",
      "La Caja se cierra al final del día con lo que efectivamente entró, y las cuentas de Banco quedan aparte para conciliar por separado.",
    ],
    steps: [
      "Vender y comprar — el asiento contable se genera solo.",
      "Cerrar Caja al final de cada jornada.",
      "Revisar el Libro Mayor por cuenta cuando lo necesites.",
    ],
    badge: "LM",
    badgeVariant: "default" as const,
    calloutTitle: "Contabilidad al día, siempre",
    calloutBody: "Ningún reporte financiero depende de que alguien se acuerde de anotarlo.",
  },
  {
    code: "NM",
    title: "Nómina",
    subtitle: "Empleados · Adelantos · Mandato",
    paragraphs: [
      "Cada empleado tiene su ficha con salario, y los adelantos que pida quedan registrados contra su próxima nómina.",
      "Mandato calcula el salario neto exacto de cualquier empleado: SFS y AFP del empleado, el tramo de ISR que le corresponde, y aparte, lo que le cuesta a la empresa — SFS, AFP, riesgos laborales e INFOTEP patronal.",
    ],
    steps: [
      "Registrar al empleado y su salario mensual.",
      "Anotar un adelanto si lo pidió.",
      "Abrir Mandato para ver el neto y el costo patronal exacto.",
    ],
    badge: "ISR",
    badgeVariant: "warning" as const,
    calloutTitle: "Los tres tramos, sin adivinar",
    calloutBody: "SFS 3.04% · AFP 2.87% del empleado — más el aporte patronal aparte.",
  },
  {
    code: "DG",
    title: "Cumplimiento DGII",
    subtitle: "e-CF · Reporte 606 · IT-1 · Auditoría",
    paragraphs: [
      "Cada comprobante fiscal electrónico se firma digitalmente al momento de la venta — no hay un paso aparte de \"generar el e-CF\" al final del día.",
      "El reporte 606 de compras sale en TXT o CSV en el formato oficial de la DGII, listo para subir. La declaración mensual de ITBIS (IT-1) muestra lo trasladado, lo acreditable y si hay que pagar o queda a favor. Y cada acción queda en Auditoría: quién, qué y cuándo.",
    ],
    steps: [
      "Generar el 606 del mes en el formato que pida la DGII.",
      "Generar el IT-1 y ver el ITBIS a pagar o a favor.",
      "Consultar Auditoría si hace falta rastrear un cambio.",
    ],
    badge: "606",
    badgeVariant: "accent" as const,
    calloutTitle: "Formato oficial, sin transcribir",
    calloutBody: "Un solo origen de datos — no hay un Excel aparte que reconciliar antes de declarar.",
  },
];

const CONTACTO_EMAIL = "manuelmat25@gmail.com";

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

export default function RecorridoPage() {
  return (
    <div className="min-h-screen bg-background">
      <style>{`@page { size: letter; margin: .5in; } @media print { .no-print { display: none !important; } .print-page { break-after: page; } .print-page:last-of-type { break-after: auto; } }`}</style>

      <header className="no-print sticky top-0 z-40 border-b border-border bg-background/80 backdrop-blur-md">
        <div className="mx-auto max-w-5xl h-16 px-5 flex items-center justify-between">
          <Link href="/" className="flex items-center gap-2 text-sm font-medium text-foreground-soft hover:text-foreground transition-colors">
            <ArrowLeft className="h-4 w-4" /> Volver
          </Link>
          <BrandMark />
          <Button variant="outline" size="sm" onClick={() => window.print()}>
            <Printer className="h-4 w-4" /> Guardar como PDF
          </Button>
        </div>
      </header>

      <section className="print-page relative overflow-hidden bg-primary text-primary-foreground">
        <div className="absolute inset-0 bg-grid-dots opacity-10 [mask-image:radial-gradient(ellipse_60%_50%_at_50%_0%,black,transparent)]" />
        <div className="relative mx-auto max-w-5xl px-5 py-16 lg:py-24">
          <p className="font-mono text-xs uppercase tracking-widest text-accent">Recorrido del sistema</p>
          <h1 className="mt-3 text-4xl sm:text-5xl font-bold tracking-tight text-wrap-balance">Colmado POS</h1>
          <p className="mt-5 max-w-xl text-lg text-primary-foreground/85 leading-relaxed">
            POS, facturación e-CF, inventario, contabilidad y nómina — todo en un mismo sistema. Esta guía recorre,
            módulo por módulo, cómo tu negocio lo usaría cada día.
          </p>
          <nav className="mt-10 grid grid-cols-2 sm:grid-cols-3 gap-x-6 gap-y-2 border-t border-primary-foreground/20 pt-5">
            {RECORRIDO.map((r) => (
              <a key={r.code} href={`#${r.code.toLowerCase()}`} className="flex items-baseline gap-2 text-sm text-primary-foreground/90 hover:text-accent transition-colors">
                <span className="font-mono text-accent text-xs">{r.code}</span>
                {r.title}
              </a>
            ))}
            <a href="#us" className="flex items-baseline gap-2 text-sm text-primary-foreground/90 hover:text-accent transition-colors">
              <span className="font-mono text-accent text-xs">US</span>
              Usuarios y Roles
            </a>
          </nav>
        </div>
      </section>

      <div className="mx-auto max-w-5xl px-5 py-14 space-y-10">
        {RECORRIDO.map((r) => (
          <Card key={r.code} id={r.code.toLowerCase()} className="print-page p-8 scroll-mt-20">
            <div className="flex items-baseline justify-between gap-4 border-b border-foreground pb-3 mb-6">
              <h2 className="text-2xl font-bold tracking-tight">{r.title}</h2>
              <span className="font-mono text-xs text-foreground-soft">{r.subtitle}</span>
            </div>
            <div className="grid grid-cols-1 lg:grid-cols-[1.6fr_1fr] gap-8">
              <div>
                {r.paragraphs.map((p, i) => (
                  <p key={i} className={`text-[15px] leading-relaxed text-foreground-soft max-w-[42ch] ${i > 0 ? "mt-3" : ""}`}>
                    {p}
                  </p>
                ))}
                <ol className="mt-6 space-y-0">
                  {r.steps.map((s, i) => (
                    <li
                      key={i}
                      className={`flex gap-3 text-sm py-2.5 border-t ${i === 0 ? "border-t-2 border-t-foreground" : "border-border"}`}
                    >
                      <span className="font-mono font-bold text-accent-hover">{String(i + 1).padStart(2, "0")}</span>
                      <span>{s}</span>
                    </li>
                  ))}
                </ol>
              </div>
              <Card className="self-start bg-muted/40 p-5">
                <Badge variant={r.badgeVariant} className="mb-3">{r.badge}</Badge>
                <h3 className="font-semibold text-sm">{r.calloutTitle}</h3>
                <p className="mt-2 text-sm text-foreground-soft leading-relaxed">{r.calloutBody}</p>
              </Card>
            </div>
          </Card>
        ))}

        <Card id="us" className="print-page p-8 bg-foreground text-background border-foreground scroll-mt-20">
          <div className="flex items-baseline justify-between gap-4 border-b border-background/25 pb-3 mb-6">
            <h2 className="text-2xl font-bold tracking-tight">Usuarios y Roles</h2>
            <span className="font-mono text-xs text-background/70">Permisos · Restablecer contraseña</span>
          </div>
          <div className="grid grid-cols-1 lg:grid-cols-[1.6fr_1fr] gap-8">
            <div>
              <p className="text-[15px] leading-relaxed text-background/85 max-w-[42ch]">
                Los roles no vienen fijos de fábrica — se arman por permiso, módulo por módulo, según lo que cada
                persona del negocio realmente necesita hacer.
              </p>
              <p className="mt-3 text-[15px] leading-relaxed text-background/85 max-w-[42ch]">
                Si alguien olvida su contraseña, no hace falta correo: se restablece con un clic, se le da una
                contraseña temporal, y el sistema lo obliga a cambiarla la primera vez que entra.
              </p>
              <ol className="mt-6 space-y-0">
                {[
                  "Armar los roles y permisos a la medida del negocio.",
                  "Restablecer una contraseña con un clic, sin correo.",
                  "El usuario la cambia obligatoriamente al entrar.",
                ].map((s, i) => (
                  <li key={i} className={`flex gap-3 text-sm py-2.5 border-t ${i === 0 ? "border-t-2 border-t-background" : "border-background/20"}`}>
                    <span className="font-mono font-bold text-accent">{String(i + 1).padStart(2, "0")}</span>
                    <span>{s}</span>
                  </li>
                ))}
              </ol>
            </div>
            <Card className="self-start bg-background/10 border-background/25 p-5">
              <Badge variant="accent" className="mb-3">OK</Badge>
              <h3 className="font-semibold text-sm">Cuenta de prueba</h3>
              <p className="mt-2 text-sm text-background/85 leading-relaxed">
                Cada nuevo cliente empieza con una cuenta de demostración limitada para explorar el sistema antes de
                decidir.
              </p>
            </Card>
          </div>
          <p className="mt-8 text-sm text-background/85">
            ¿Quieres verlo funcionando con tu propio negocio?{" "}
            <a href={`mailto:${CONTACTO_EMAIL}`} className="text-accent hover:underline font-mono">
              {CONTACTO_EMAIL}
            </a>
          </p>
        </Card>
      </div>
    </div>
  );
}
