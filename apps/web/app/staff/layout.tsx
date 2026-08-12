import type { Metadata } from "next";
import { Public_Sans, JetBrains_Mono } from "next/font/google";
import "./globals.css";

const publicSans = Public_Sans({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  display: "swap",
  variable: "--font-sans",
});

// Un solo tipo de letra para todo (a diferencia del layout del cliente,
// que combina serif + sans para transmitir confianza institucional) -
// esta es una consola interna, la prioridad es legibilidad densa, no
// identidad de marca. Mono aparte para RNC, códigos de módulo y claves.
const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  weight: ["400", "500", "600"],
  display: "swap",
  variable: "--font-mono",
});

export const metadata: Metadata = {
  title: "Staff · Colmado POS",
  description: "Panel interno de ventas y onboarding — no es el producto para clientes.",
  robots: { index: false, follow: false },
};

export default function StaffRootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="es-DO" suppressHydrationWarning>
      <body
        suppressHydrationWarning
        className={`${publicSans.variable} ${jetbrainsMono.variable} font-sans min-h-screen bg-background text-foreground antialiased selection:bg-primary selection:text-primary-foreground`}
      >
        {children}
      </body>
    </html>
  );
}
