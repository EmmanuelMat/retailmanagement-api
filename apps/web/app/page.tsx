import Link from "next/link";

export default function Landing() {
  return (
    <div className="min-h-screen bg-[#0c0a09] text-white flex items-center justify-center p-6">
      <div className="max-w-[800px] w-full rounded-[24px] border-[4px] border-black bg-white text-black p-8 shadow-[12px_12px_0px_black]">
        <div className="flex items-center gap-4">
          <div className="h-16 w-16 rounded-[16px] bg-amber-400 border-[3px] border-black flex items-center justify-center text-3xl">☀️</div>
          <div>
            <h1 className="font-black text-[28px] leading-none tracking-tight">COLMADO EL SOL • SISTEMA COMPLETO</h1>
            <p className="font-mono text-[11px] mt-1 opacity-60">RNC 130-79375-2 • Núcleo Bancario Rust + TigerBeetle • DGII e-CF • Español 100% • 14 Módulos</p>
          </div>
        </div>
        
        <div className="mt-6 grid grid-cols-2 gap-3">
          <Link href="/pos" className="rounded-[14px] border-[3px] border-black bg-black text-white p-4 shadow-[4px_4px_0px_black] hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[6px_6px_0px_black] transition-all">
            <p className="font-black text-[14px]">🛒 IR A POS • Terminal Venta</p>
            <p className="text-[11px] opacity-60 mt-1">Diseño único colmado hand-painted, receipt zigzag, bento cards tilt, post-it adelantos</p>
          </Link>
          <Link href="/" className="rounded-[14px] border-[3px] border-black bg-amber-400 text-black p-4 shadow-[4px_4px_0px_black] hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[6px_6px_0px_black] transition-all">
            <p className="font-black text-[14px]">📊 Dashboard Gerencial • 14 Módulos</p>
            <p className="text-[11px] opacity-70 mt-1">Ventas hoy, ITBIS, top productos, cumplimiento DGII, ledger vivo</p>
          </Link>
        </div>

        <div className="mt-6 rounded-[12px] border-2 border-dashed border-black/20 bg-amber-50 p-4">
          <p className="font-black text-[12px]">📚 PLAN MAESTRO • 14 MÓDULOS EN ESPAÑOL</p>
          <p className="text-[11px] mt-2 leading-relaxed">1. Auth • 2. Productos • 3. Inventario • 4. Clientes/Proveedores • 5. POS + Caja • 6. Compras 606 • 7. Contabilidad • 8. Nómina + Adelantos 50% • 9. Caja y Bancos • 10. Reportes DGII 606/607/IT-1 • 11. Config DGII • 12. Móvil • 13. RFCE + ARECF/ACECF • 14. SaaS Billing. Ver docs/00-PLAN-MAESTRO-SISTEMA-COMPLETO.md</p>
        </div>

        <div className="mt-6 flex gap-2">
          <Link href="/inventario/productos" className="text-[11px] font-black bg-stone-900 text-white border-2 border-black rounded-full px-3 py-1.5">Productos CRUD</Link>
          <Link href="/clientes" className="text-[11px] font-black bg-stone-900 text-white border-2 border-black rounded-full px-3 py-1.5">Clientes RNC</Link>
          <Link href="/nomina/adelantos" className="text-[11px] font-black bg-amber-400 text-black border-2 border-black rounded-full px-3 py-1.5">Adelantos EWA 50%</Link>
          <Link href="/reportes/dgii" className="text-[11px] font-black bg-sky-400 text-black border-2 border-black rounded-full px-3 py-1.5">Reportes DGII</Link>
          <Link href="/configuracion/dgii" className="text-[11px] font-black bg-white text-black border-2 border-black rounded-full px-3 py-1.5">Config DGII</Link>
        </div>
      </div>
    </div>
  );
}
