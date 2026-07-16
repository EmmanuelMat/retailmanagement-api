import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Colmado POS Dominicana - Sistema Bancario | DGII e-CF | Contabilidad | Adelantos",
  description: "POS con núcleo bancario en Rust para PYMES Dominicanas. Cumple DGII Facturación Electrónica e-CF, inventario con TigerBeetle, contabilidad completa, nómina con adelantos 50%. Event-driven como banco.",
  keywords: ["POS", "DGII", "e-CF", "Dominicana", "Colmado", "Inventario", "Contabilidad", "Adelantos", "Nómina", "TigerBeetle", "Rust"],
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="es-DO">
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
      </head>
      <body className="min-h-screen bg-zinc-950 text-zinc-100 antialiased selection:bg-orange-500 selection:text-white">
        {children}
      </body>
    </html>
  );
}
