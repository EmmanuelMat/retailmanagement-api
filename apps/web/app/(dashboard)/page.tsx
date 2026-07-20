export default function DashboardGerencial() {
  return (
    <div className="p-6 space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="font-black text-[28px] tracking-tight">Dashboard Gerencial • Colmado El Sol</h1>
          <p className="text-[12px] font-mono text-stone-400 mt-1">Hoy • 19 Julio 2026 • RNC 130-79375-2 • TigerBeetle vivo • DGII e-CF activo</p>
        </div>
        <div className="flex gap-2">
          <span className="bg-emerald-400 text-black border-2 border-black rounded-full px-3 py-1 text-[11px] font-black shadow-[3px_3px_0px_black]">● EN VIVO • 47 VENTAS HOY</span>
        </div>
      </div>

      <div className="grid grid-cols-4 gap-4">
        {[
          { label: "Ventas Hoy", valor: "RD$ 18,420.00", sub: "+12% vs ayer • Ticket promedio RD$ 392", color: "bg-white text-black border-black", icon: "💰" },
          { label: "ITBIS Cobrado", valor: "RD$ 3,211.00", sub: "Para IT-1 • 18% RD$2,800 • 16% RD$411", color: "bg-sky-400 text-black border-black", icon: "🧾" },
          { label: "Ganancia Neta Hoy", valor: "RD$ 5,830.00", sub: "Ingresos - Costos - Gastos • 31.6% margen", color: "bg-amber-400 text-black border-black", icon: "📈" },
          { label: "Productos Bajo Mínimo", valor: "3 productos", sub: "Coca-Cola 2L, Aceite Crisol, Detergente", color: "bg-red-400 text-black border-black", icon: "⚠️" },
        ].map((c) => (
          <div key={c.label} className={`rounded-[16px] border-[3px] ${c.color} p-4 shadow-[6px_6px_0px_black]`}>
            <div className="flex justify-between">
              <p className="font-black text-[11px] tracking-widest">{c.label}</p>
              <span className="text-[18px]">{c.icon}</span>
            </div>
            <p className="font-black text-[22px] mt-2 tracking-tight">{c.valor}</p>
            <p className="text-[10px] font-bold opacity-70 mt-1">{c.sub}</p>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-3 gap-4">
        <div className="col-span-2 rounded-[16px] border-[3px] border-black bg-stone-900 p-4 shadow-[6px_6px_0px_black]">
          <h3 className="font-black text-[13px]">Ventas por Hora • Hoy (Gráfico)</h3>
          <div className="mt-4 h-32 flex items-end gap-2">
            {[40,65,30,85,45,90,70,55,80,60,75,50].map((h,i) => (
              <div key={i} className="flex-1 bg-amber-400 border-2 border-black rounded-t-[6px]" style={{height: `${h}%`}} />
            ))}
          </div>
          <p className="text-[10px] font-mono text-stone-500 mt-2">Pico: 6pm cierre • 90% ventas entre 4pm-8pm • TigerBeetle 1,284 events hoy</p>
        </div>
        <div className="rounded-[16px] border-[3px] border-black bg-white text-black p-4 shadow-[6px_6px_0px_black]">
          <h3 className="font-black text-[12px]">Top Productos • Hoy</h3>
          <div className="mt-3 space-y-2 text-[11px] font-black">
            <div className="flex justify-between"><span>1. Arroz Premium</span><span>RD$ 2,360</span></div>
            <div className="flex justify-between"><span>2. Plátanos</span><span>RD$ 1,800</span></div>
            <div className="flex justify-between"><span>3. Coca-Cola 2L</span><span>RD$ 1,140</span></div>
            <div className="flex justify-between"><span>4. Pan Sobao</span><span>RD$ 600</span></div>
          </div>
          <p className="text-[9px] font-mono mt-3 opacity-50">Desde read_sales agregado • No mock</p>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="rounded-[16px] border-[3px] border-black bg-amber-100 text-black p-4 shadow-[6px_6px_0px_black]">
          <h3 className="font-black text-[12px]">⚡ Acciones Rápidas • Módulos</h3>
          <div className="mt-3 grid grid-cols-3 gap-2">
            {["Nueva Venta POS","Nuevo Producto","Nuevo Cliente","Nueva Compra","Correr Nómina","Exportar 606/607","Abrir Caja","Cerrar Caja"].map((a) => (
              <button key={a} className="bg-black text-white border-2 border-black rounded-full py-2 text-[10px] font-black hover:bg-white hover:text-black transition-colors">{a}</button>
            ))}
          </div>
        </div>
        <div className="rounded-[16px] border-[3px] border-black bg-black text-white p-4 shadow-[6px_6px_0px_white]">
          <h3 className="font-black text-[12px] text-stone-400">Cumplimiento DGII • Hoy</h3>
          <div className="mt-3 space-y-2 text-[11px] font-mono">
            <div className="flex justify-between"><span>E32 &lt;250k generados</span><span className="text-emerald-400 font-black">47 • RFCE pendiente cierre</span></div>
            <div className="flex justify-between"><span>E31 Crédito Fiscal</span><span className="text-sky-400">3 • 3 Aceptados DGII</span></div>
            <div className="flex justify-between"><span>TrackID pendientes</span><span className="text-amber-400">1 • dgi-982x EnProceso</span></div>
            <div className="flex justify-between"><span>606/607 listo</span><span className="text-white font-black">TXT pre-validado OK</span></div>
          </div>
        </div>
      </div>
    </div>
  );
}
