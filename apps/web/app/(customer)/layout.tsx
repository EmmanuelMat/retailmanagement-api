import type { Metadata } from "next";
import { Public_Sans, Source_Serif_4 } from "next/font/google";
import "./globals.css";

const publicSans = Public_Sans({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  display: "swap",
  variable: "--font-sans",
});

const sourceSerif = Source_Serif_4({
  subsets: ["latin"],
  weight: ["600"],
  display: "swap",
  variable: "--font-serif",
});

export const metadata: Metadata = {
  title: "Colmado POS Dominicana - DGII e-CF | Inventario | Contabilidad",
  description: "Sistema de gestión para PYMES Dominicanas. Facturación Electrónica DGII e-CF, inventario, contabilidad y nómina en un núcleo Rust.",
  keywords: ["POS", "DGII", "e-CF", "Dominicana", "Colmado", "Inventario", "Contabilidad", "Nómina", "Rust"],
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="es-DO" suppressHydrationWarning>
      <body
        suppressHydrationWarning
        className={`${publicSans.variable} ${sourceSerif.variable} font-sans min-h-screen bg-background text-foreground antialiased selection:bg-primary selection:text-primary-foreground`}
      >
        {children}
      </body>
    </html>
  );
}
