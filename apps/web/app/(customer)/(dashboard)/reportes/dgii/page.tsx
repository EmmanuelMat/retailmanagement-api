import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { Card, CardContent } from "@repo/ui";

const REPORTES = [
  { label: "606 · Compras", href: "/reportes/dgii/606", desc: "TXT de compras del período" },
  { label: "IT-1", href: "/reportes/dgii/it1", desc: "Declaración mensual de ITBIS" },
];

export default function ReportesDgiiPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Reportes DGII</h1>
        <p className="text-sm text-muted-foreground mt-1">Formatos de cumplimiento fiscal, generados desde tus ventas y compras reales.</p>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {REPORTES.map((r) => (
          <Link key={r.href} href={r.href as any}>
            <Card className="h-full hover:border-primary transition-colors">
              <CardContent className="pt-5 flex items-center justify-between">
                <div>
                  <p className="text-sm font-semibold">{r.label}</p>
                  <p className="text-xs text-muted-foreground mt-1">{r.desc}</p>
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
