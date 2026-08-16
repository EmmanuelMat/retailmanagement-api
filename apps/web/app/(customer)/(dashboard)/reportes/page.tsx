import Link from "next/link";
import { FileText, TrendingUp, Package, DollarSign, ArrowRight } from "lucide-react";
import { Card, CardContent } from "@repo/ui";

const REPORTES = [
  { label: "DGII (606)", href: "/reportes/dgii", icon: FileText, desc: "Reportes fiscales para la DGII" },
  { label: "Ventas", href: "/reportes/ventas", icon: TrendingUp, desc: "Historial detallado — ver Ventas" },
  { label: "Inventario", href: "/reportes/inventario", icon: Package, desc: "Valorización y kardex — ver Movimientos" },
  { label: "Financiero", href: "/reportes/financiero", icon: DollarSign, desc: "Libro mayor — ver Contabilidad" },
];

export default function ReportesPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Reportes</h1>
        <p className="text-sm text-muted-foreground mt-1">Fiscales, ventas, inventario y financieros.</p>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {REPORTES.map((r) => (
          <Link key={r.href} href={r.href as any}>
            <Card className="hover:border-primary transition-colors">
              <CardContent className="pt-5 flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="h-9 w-9 rounded-md bg-primary/10 text-primary flex items-center justify-center"><r.icon className="h-4.5 w-4.5" /></div>
                  <div>
                    <p className="text-sm font-semibold">{r.label}</p>
                    <p className="text-xs text-muted-foreground">{r.desc}</p>
                  </div>
                </div>
                <ArrowRight className="h-4 w-4 text-muted-foreground" />
              </CardContent>
            </Card>
          </Link>
        ))}
      </div>
    </div>
  );
}
