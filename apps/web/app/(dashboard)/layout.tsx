import Link from "next/link";

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-[#0c0a09] text-stone-100">
      <header className="border-b-[3px] border-stone-800 bg-[#1c1917] sticky top-0 z-20">
        <div className="flex items-center justify-between px-5 py-3">
          <div className="flex items-center gap-3">
            <div className="h-10 w-10 rounded-[12px] bg-amber-400 border-[3px] border-black flex items-center justify-center font-black">☀️</div>
            <h1 className="font-black text-[14px] tracking-tight">COLMADO EL SOL • SISTEMA COMPLETO</h1>
            <span className="text-[10px] bg-white text-black border-2 border-black rounded-full px-2 py-0.5 font-black">14 MÓDULOS • ESPAÑOL 100%</span>
          </div>
          <div className="flex gap-2 text-[11px] font-black">
            <Link href="/pos" className="bg-white text-black border-2 border-black rounded-full px-3 py-1">POS</Link>
            <Link href="/inventario/productos" className="bg-stone-900 border-2 border-stone-800 rounded-full px-3 py-1">Productos</Link>
            <Link href="/reportes/dgii" className="bg-sky-400 text-black border-2 border-black rounded-full px-3 py-1">DGII</Link>
          </div>
        </div>
      </header>
      <div className="flex">
        <aside className="hidden lg:block w-[220px] border-r-2 border-stone-800 bg-stone-900/50 p-3 space-y-1">
          {[
            ["Dashboard", "/", "📊"],
            ["POS Venta", "/pos", "🛒"],
            ["Inventario", "/inventario", "📦"],
            ["Productos", "/inventario/productos", "🏷️"],
            ["Clientes", "/clientes", "👥"],
            ["Proveedores", "/proveedores", "🚚"],
            ["Compras 606", "/compras", "📥"],
            ["Ventas", "/ventas", "💳"],
            ["Contabilidad", "/contabilidad", "📒"],
            ["Nómina", "/nomina", "💼"],
            ["Adelantos", "/nomina/adelantos", "💸"],
            ["Caja", "/caja", "💰"],
            ["Bancos", "/bancos", "🏦"],
            ["Reportes DGII", "/reportes/dgii", "🇩🇴"],
            ["Config DGII", "/configuracion/dgii", "🔐"],
          ].map(([label, href, icon]) => (
            <Link key={href} href={href as any} className="flex items-center gap-2 px-3 py-2 rounded-full border-2 border-transparent hover:border-stone-700 hover:bg-stone-800 text-[12px] font-bold">
              <span>{icon}</span>{label}
            </Link>
          ))}
        </aside>
        <main className="flex-1 bg-[#0c0a09]">{children}</main>
      </div>
    </div>
  );
}
