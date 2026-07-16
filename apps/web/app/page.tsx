/**
 * POS Banco-Grado - Interfaz en Español Dominicano
 * Núcleo Rust + TigerBeetle + DGII e-CF
 */

export default function PaginaPrincipal() {
  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-100">
      {/* Encabezado */}
      <header className="sticky top-0 z-20 border-b border-zinc-800 bg-zinc-900/80 backdrop-blur-xl">
        <div className="mx-auto flex max-w-[1600px] items-center justify-between px-6 py-4">
          <div className="flex items-center gap-3">
            <div className="h-10 w-10 rounded-xl bg-gradient-to-br from-orange-500 to-red-600 flex items-center justify-center font-black text-sm shadow-lg shadow-orange-500/20">RD</div>
            <div>
              <h1 className="font-bold tracking-tight text-[15px]">Colmado POS • Núcleo Bancario</h1>
              <p className="text-[11px] text-zinc-400">Eventos • TigerBeetle • DGII e-CF • Contabilidad • Nómina</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <span className="rounded-full bg-emerald-500/10 px-3 py-1 text-[11px] text-emerald-400 border border-emerald-500/20">● Núcleo Rust: Conectado</span>
            <span className="rounded-full bg-zinc-800 px-3 py-1 text-[11px] text-zinc-300">RNC: 130793752 • Colmado El Sol • SDO</span>
            <div className="ml-2 h-8 w-8 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center text-xs font-bold">EM</div>
          </div>
        </div>
      </header>

      {/* Navegación rápida */}
      <div className="mx-auto max-w-[1600px] px-6 pt-4 flex gap-2 text-[12px]">
        {[
          { label: "Punto de Venta", active: true },
          { label: "Inventario" },
          { label: "Clientes" },
          { label: "Contabilidad" },
          { label: "Nómina y Adelantos" },
          { label: "Reportes DGII (606/607)" },
          { label: "Configuración DGII" },
        ].map((tab) => (
          <div key={tab.label} className={`rounded-full px-4 py-1.5 border cursor-pointer transition ${tab.active ? "bg-white text-black border-white font-semibold" : "bg-zinc-900 border-zinc-800 text-zinc-400 hover:border-zinc-700"}`}>
            {tab.label}
          </div>
        ))}
      </div>

      <div className="mx-auto max-w-[1600px] grid grid-cols-12 gap-6 p-6">
        {/* Izquierda - Terminal de Venta */}
        <div className="col-span-8 space-y-5">
          {/* Métricas superiores */}
          <div className="grid grid-cols-4 gap-4">
            {[
              { label: "Caja de Hoy", valor: "RD$ 18,420.00", detalle: "+12% vs ayer • 47 ventas", color: "text-emerald-400", bg: "from-emerald-500/10 to-emerald-600/5" },
              { label: "Facturas DGII", valor: "47 E32", detalle: "46 Aceptadas • 1 Pendiente TrackID", color: "text-sky-400", bg: "from-sky-500/10 to-blue-600/5" },
              { label: "Adelantos Activos", valor: "RD$ 6,000.00", detalle: "3 empleados • 50% regla", color: "text-amber-400", bg: "from-amber-500/10 to-orange-600/5" },
              { label: "Eventos Ledger", valor: "1,284", detalle: "Inmutable • Hash encadenado", color: "text-violet-400", bg: "from-violet-500/10 to-purple-600/5" },
            ].map((m) => (
              <div key={m.label} className={`rounded-2xl border border-zinc-800 bg-gradient-to-br ${m.bg} bg-zinc-900 p-4`}>
                <p className="text-[11px] uppercase tracking-wider text-zinc-400">{m.label}</p>
                <p className={`mt-1 text-[18px] font-bold ${m.color}`}>{m.valor}</p>
                <p className="mt-1 text-[11px] text-zinc-500">{m.detalle}</p>
              </div>
            ))}
          </div>

          {/* Terminal POS */}
          <div className="rounded-2xl border border-zinc-800 bg-zinc-900 overflow-hidden">
            <div className="border-b border-zinc-800 p-4 flex justify-between items-center bg-zinc-900/50">
              <div>
                <h2 className="font-semibold text-[14px]">Terminal de Venta • Consumidor Final</h2>
                <p className="text-[11px] text-zinc-400 mt-0.5">Tipo e-CF: E32 ( &lt; RD$250,000 ) • Modo Resumen RFCE • ITBIS 18% / 16% / Exento</p>
              </div>
              <div className="flex gap-2">
                <span className="text-[11px] bg-zinc-800 border border-zinc-700 px-2.5 py-1 rounded-lg">eNCF: E320000000128</span>
                <span className="text-[11px] bg-sky-950 border border-sky-900 text-sky-300 px-2.5 py-1 rounded-lg">RFCE: Activo</span>
              </div>
            </div>
            
            <div className="grid grid-cols-5 gap-6 p-4">
              {/* Productos */}
              <div className="col-span-3">
                <div className="flex justify-between items-center mb-3">
                  <h3 className="text-[12px] font-semibold uppercase tracking-wider text-zinc-400">Productos • Inventario en TigerBeetle</h3>
                  <input placeholder="Buscar por SKU o nombre..." className="bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-1.5 text-xs w-56 focus:outline-none focus:border-zinc-700" />
                </div>
                <div className="grid grid-cols-2 gap-3">
                  {[
                    { sku: "PLT-001", nombre: "Plátanos x libra", precio: "45.00", itbis: "EXENTO", existencia: 42, categoria: "Víveres" },
                    { sku: "ARZ-002", nombre: "Arroz Premium 1lb", precio: "118.00", itbis: "18% • Gravado", existencia: 18, categoria: "Víveres" },
                    { sku: "REF-010", nombre: "Coca-Cola 2L", precio: "95.00", itbis: "18% • Gravado", existencia: 3, categoria: "Bebidas", alerta: true },
                    { sku: "PAN-001", nombre: "Pan Sobao", precio: "10.00", itbis: "16% • Reducida", existencia: 120, categoria: "Panadería" },
                    { sku: "ACE-005", nombre: "Aceite Crisol 16oz", precio: "185.00", itbis: "16% • Reducida", existencia: 24, categoria: "Víveres" },
                    { sku: "DET-003", nombre: "Detergente Ace", precio: "135.00", itbis: "18% • Gravado", existencia: 15, categoria: "Limpieza" },
                  ].map((p) => (
                    <div key={p.sku} className={`group cursor-pointer rounded-xl border p-3 transition ${p.alerta ? "border-amber-900/50 bg-amber-950/20 hover:border-amber-800" : "border-zinc-800 bg-zinc-950 hover:border-zinc-700 hover:bg-zinc-900"}`}>
                      <div className="flex justify-between items-start">
                        <p className="text-[10px] text-zinc-500">{p.sku} • {p.categoria}</p>
                        <span className={`text-[10px] px-1.5 py-0.5 rounded ${p.existencia < 5 ? "bg-red-950 text-red-300 border border-red-900" : "bg-zinc-900 text-zinc-400 border border-zinc-800"}`}>Exist: {p.existencia}</span>
                      </div>
                      <p className="font-medium mt-1 text-[13px] group-hover:text-white">{p.nombre}</p>
                      <div className="mt-2 flex justify-between items-center">
                        <p className="text-[13px] font-bold">RD$ {p.precio}</p>
                        <span className="text-[9px] bg-zinc-900 border border-zinc-800 px-1.5 py-0.5 rounded text-zinc-400">{p.itbis}</span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Carrito */}
              <div className="col-span-2 rounded-xl bg-zinc-950 border border-zinc-800 p-4 flex flex-col">
                <h3 className="text-[12px] font-semibold uppercase tracking-wider text-zinc-400">Carrito de Venta</h3>
                
                <div className="mt-3 space-y-2">
                  <div className="flex justify-between text-[12px] bg-zinc-900 border border-zinc-800 rounded-lg p-2">
                    <span>Plátanos x 2 lb</span><span>RD$ 90.00</span>
                  </div>
                  <div className="flex justify-between text-[12px] bg-zinc-900 border border-zinc-800 rounded-lg p-2">
                    <span>Arroz Premium (1)</span><span>RD$ 118.00</span>
                  </div>
                  <div className="flex justify-between text-[12px] bg-zinc-900 border border-zinc-800 rounded-lg p-2">
                    <span>Coca-Cola 2L (1)</span><span>RD$ 95.00</span>
                  </div>
                </div>

                <div className="mt-auto pt-4 space-y-1.5">
                  <div className="flex justify-between text-[12px] text-zinc-400">
                    <span>Subtotal Gravado 18%</span><span>RD$ 213.00</span>
                  </div>
                  <div className="flex justify-between text-[12px] text-zinc-400">
                    <span>Exento + 16% Reducida</span><span>RD$ 90.00</span>
                  </div>
                  <div className="flex justify-between text-[12px] text-zinc-400">
                    <span>ITBIS 18%</span><span>RD$ 38.34</span>
                  </div>
                  <div className="flex justify-between font-bold text-[16px] border-t border-zinc-800 pt-3">
                    <span>Total a Pagar</span><span>RD$ 341.34</span>
                  </div>
                  <div className="grid grid-cols-2 gap-2 mt-3">
                    <button className="rounded-xl bg-zinc-800 border border-zinc-700 text-sm py-2.5 hover:bg-zinc-700 transition text-zinc-200">Efectivo</button>
                    <button className="rounded-xl bg-zinc-800 border border-zinc-700 text-sm py-2.5 hover:bg-zinc-700 transition text-zinc-200">Tarjeta</button>
                  </div>
                  <button className="w-full rounded-xl bg-white text-black font-bold py-3 hover:bg-zinc-200 transition shadow-lg shadow-white/10">
                    Cobrar • Generar E32 • QR DGII
                  </button>
                  <p className="text-[10px] text-zinc-500 text-center leading-tight">
                    Flujo bancario: Evento VentaCompletada → Núcleo Rust → TigerBeetle (Reserva Inventario + Linked Transfers) → Firma XAdES-BES → DGII Asíncrono (TrackID)
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Derecha - Contabilidad + Adelantos */}
        <div className="col-span-4 space-y-5">
          <div className="rounded-2xl border border-zinc-800 bg-zinc-900 p-4">
            <div className="flex justify-between items-center">
              <h3 className="font-semibold text-[13px]">Libro Mayor • TigerBeetle (Vivo)</h3>
              <span className="text-[10px] bg-emerald-950 border border-emerald-900 text-emerald-300 px-2 py-0.5 rounded-full">Tiempo Real</span>
            </div>
            <div className="mt-4 space-y-2.5 text-[12px]">
              <div className="flex justify-between"><span className="text-zinc-400">activo:caja_efectivo</span><span className="font-mono font-semibold">RD$ 18,420.00</span></div>
              <div className="flex justify-between"><span className="text-zinc-400">activo:banco_popular</span><span className="font-mono">RD$ 42,100.00</span></div>
              <div className="flex justify-between"><span className="text-amber-300">activo:anticipos_empleados</span><span className="font-mono text-amber-300">RD$ 6,000.00</span></div>
              <div className="flex justify-between"><span className="text-sky-300">pasivo:itbis_por_pagar</span><span className="font-mono text-sky-300">RD$ 4,210.00</span></div>
              <div className="flex justify-between"><span className="text-zinc-400">pasivo:tss_por_pagar</span><span className="font-mono">RD$ 1,180.00</span></div>
              <div className="flex justify-between border-t border-zinc-800 pt-2"><span className="text-zinc-200 font-semibold">ingreso:ventas_gravadas</span><span className="font-mono text-emerald-400 font-bold">RD$ 42,100.00</span></div>
            </div>
            <p className="text-[10px] text-zinc-500 mt-3 leading-tight">Balances calculados desde TigerBeetle: debits_posted - credits_posted. Inmutable, sin UPDATE. Atomicidad con transfers vinculados.</p>
          </div>

          <div className="rounded-2xl border border-amber-900/50 bg-gradient-to-br from-amber-950/30 to-orange-950/20 p-4">
            <div className="flex justify-between items-center">
              <h3 className="font-semibold text-[13px] text-amber-200">Adelantos Hoy • Préstamos EWA (Regla 50%)</h3>
              <span className="text-[10px] bg-amber-500 text-black font-bold px-2 py-0.5 rounded-full">3 activos</span>
            </div>
            <p className="text-[11px] text-amber-200/70 mt-1">No es préstamo. Es sueldo ya ganado. Sin interés. Descuento automático en quincena.</p>
            <div className="mt-4 space-y-3">
              {[
                { nombre: "María Pérez • Cajera", ganado: "9,200", disponible: "4,600", solicitado: "2,000", estado: "Aprobado • Transfer 3001 posted", motivo: "Medicina" },
                { nombre: "Juan Carlos • Almacén", ganado: "6,400", disponible: "3,200", solicitado: "3,000", estado: "Pendiente gerente", motivo: "Transporte" },
                { nombre: "Luisa Gómez • Limpieza", ganado: "5,100", disponible: "2,550", solicitado: "1,000", estado: "Aprobado • posted", motivo: "Útiles escolares" },
              ].map((e) => (
                <div key={e.nombre} className="rounded-xl bg-zinc-900/80 border border-amber-900/30 p-3">
                  <p className="text-[12px] font-semibold">{e.nombre}</p>
                  <p className="text-[10px] text-zinc-400 mt-1">Ganado: RD$ {e.ganado} • Disponible 50%: RD$ {e.disponible} • Motivo: {e.motivo}</p>
                  <div className="mt-2 flex justify-between items-center">
                    <span className="text-[12px] font-bold">Solicita RD$ {e.solicitado}</span>
                    <span className="text-[9px] bg-zinc-800 border border-zinc-700 px-2 py-0.5 rounded-full text-amber-300">{e.estado}</span>
                  </div>
                  <p className="text-[9px] text-zinc-500 mt-2">Evento: AdelantoAprobado → TB: Debe activo:anticipos / Haber activo:caja (pending→posted). Deducción en nómina como “Anticipo de Salario”.</p>
                </div>
              ))}
            </div>
            <div className="mt-4 grid grid-cols-2 gap-2">
              <button className="rounded-xl bg-amber-500 text-black text-[12px] font-bold py-2 hover:bg-amber-400 transition">Aprobar Todos</button>
              <button className="rounded-xl bg-zinc-900 border border-zinc-800 text-[12px] py-2 hover:bg-zinc-800 transition">Ver Nómina</button>
            </div>
          </div>

          <div className="rounded-2xl border border-zinc-800 bg-zinc-900 p-4">
            <h3 className="font-semibold text-[12px] uppercase tracking-wider text-zinc-400">Registro de Eventos • Sólo Agrega (Inmutable)</h3>
            <div className="mt-3 space-y-2 text-[10px] font-mono leading-relaxed">
              <div className="text-zinc-500">#1824 21:33:10 <span className="text-zinc-200">VentaCompletada</span> E320000000127 • TB id 9812 linked (3 transfers atómicos)</div>
              <div className="text-zinc-500">#1825 21:33:11 <span className="text-sky-300">SolicitudFirmaTicket</span> → Núcleo Rust XAdES-BES</div>
              <div className="text-zinc-500">#1826 21:33:14 <span className="text-emerald-300">TicketAceptado</span> TrackID dgi-982x • QR https://ecf.dgii.gov.do/ • CodSeg A1B2C3</div>
              <div className="text-zinc-500">#1827 21:35:02 <span className="text-amber-300">AdelantoSolicitado</span> María RD$2,000 • Regla 50% OK • TB pending</div>
              <div className="text-zinc-500">#1828 21:35:30 <span className="text-amber-300">AdelantoAprobado</span> TB transfer 3001 pending→posted • Caja -2000</div>
              <div className="text-zinc-500">#1829 21:36:01 <span className="text-violet-300">InventarioReservado</span> SKU ARZ-002 qty 1 • flags pending</div>
            </div>
            <p className="text-[10px] text-zinc-500 mt-3">Cada evento tiene hash SHA256(prev_hash + payload). Detección de alteración como blockchain. Re-play para auditoría DGII.</p>
          </div>

          <div className="rounded-2xl border border-zinc-800 bg-zinc-900 p-4">
            <h3 className="font-semibold text-[12px]">Cumplimiento DGII • Reportes Automáticos</h3>
            <p className="text-[11px] text-zinc-400 mt-1.5">606 Compras + 607 Ventas + 608 Anulados + RFCE. Generado desde libro mayor, no tablas mutables.</p>
            <div className="mt-3 grid grid-cols-3 gap-2">
              <button className="rounded-lg bg-zinc-800 border border-zinc-700 text-[11px] py-2 hover:bg-zinc-700 transition">Exportar 606</button>
              <button className="rounded-lg bg-zinc-800 border border-zinc-700 text-[11px] py-2 hover:bg-zinc-700 transition">607 + RFCE</button>
              <button className="rounded-lg bg-zinc-800 border border-zinc-700 text-[11px] py-2 hover:bg-zinc-700 transition">IT-1</button>
            </div>
            <div className="mt-3 rounded-lg bg-sky-950/30 border border-sky-900/50 p-2.5">
              <p className="text-[10px] text-sky-300">e-CF obligatorio desde 15 Nov 2026 para pequeños. E32 &lt; RD$250k se reporta en resumen diario RFCE para no saturar DGII.</p>
            </div>
          </div>
        </div>
      </div>

      {/* Pie con estado */}
      <footer className="border-t border-zinc-800 bg-zinc-900/50 mt-4">
        <div className="mx-auto max-w-[1600px] px-6 py-3 flex justify-between items-center text-[11px] text-zinc-500">
          <span>Monorepo: apps/web (Next.js) + apps/mobile (Expo) + services/core (Rust) + TigerBeetle • EventStore Postgres</span>
          <span>Hecho en 🇩🇴 Santo Domingo • Núcleo bancario • Listo para PSFE DGII</span>
        </div>
      </footer>
    </div>
  );
}
