/**
 * COLMADO EL SOL - POS Dominicano Único
 * Diseño inspirado en letreros pintados a mano de colmados, no dashboard corporativo genérico
 * 100% Español Dominicano
 */

export default function ColmadoPOS() {
  return (
    <div className="min-h-screen bg-[#0c0a09] text-stone-100 selection:bg-amber-500 selection:text-black relative overflow-hidden">
      {/* Fondos radiales únicos */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute -top-[40%] -left-[20%] w-[80%] h-[80%] rounded-full bg-gradient-to-br from-amber-500/[0.08] to-orange-600/[0.03] blur-[120px]" />
        <div className="absolute -bottom-[30%] -right-[10%] w-[60%] h-[60%] rounded-full bg-gradient-to-br from-emerald-500/[0.06] to-teal-600/[0.02] blur-[100px]" />
        <div className="absolute top-[20%] right-[30%] w-[40%] h-[40%] rounded-full bg-gradient-to-br from-violet-500/[0.04] to-purple-600/[0.01] blur-[80px]" />
      </div>

      {/* Grid sutil */}
      <div className="fixed inset-0 pointer-events-none opacity-[0.02]" style={{
        backgroundImage: `linear-gradient(#fff 1px, transparent 1px), linear-gradient(90deg, #fff 1px, transparent 1px)`,
        backgroundSize: '48px 48px'
      }} />

      {/* Header - Hand painted style */}
      <header className="relative z-20 border-b-[3px] border-stone-800 bg-[#1c1917] sticky top-0">
        <div className="mx-auto max-w-[1800px] flex items-center justify-between px-5 py-3">
          <div className="flex items-center gap-5">
            {/* Logo sol */}
            <div className="flex items-center gap-3">
              <div className="relative h-12 w-12 rounded-[14px] bg-[#facc15] border-[3px] border-stone-900 flex items-center justify-center shadow-[4px_4px_0px_#0c0a09] rotate-[-2deg]">
                <span className="text-[22px]">☀️</span>
                <div className="absolute -top-1 -right-1 h-3 w-3 bg-emerald-500 rounded-full border-2 border-stone-900 animate-pulse" />
              </div>
              <div>
                <h1 className="font-black text-[17px] tracking-[-0.02em] leading-none">COLMADO EL SOL</h1>
                <div className="flex items-center gap-2 mt-0.5">
                  <span className="text-[10px] font-bold bg-amber-400 text-black px-1.5 py-0.5 rounded-[4px] border border-black">DESDE 1998</span>
                  <span className="text-[10px] text-stone-400 font-mono">RNC 130-79375-2 • SDO • RD</span>
                </div>
              </div>
            </div>

            <div className="hidden lg:flex items-center gap-2 ml-6">
              {[
                { icon: "🛒", label: "VENTA", active: true },
                { icon: "📦", label: "INVENTARIO", active: false },
                { icon: "👥", label: "CLIENTES", active: false },
                { icon: "📒", label: "LIBRO", active: false },
                { icon: "💸", label: "NÓMINA", active: false },
              ].map((tab) => (
                <div key={tab.label} className={`flex items-center gap-1.5 px-3.5 py-2 rounded-full border-[2.5px] text-[11px] font-black tracking-wider cursor-pointer transition-all ${tab.active ? "bg-white text-black border-black shadow-[3px_3px_0px_black]" : "bg-stone-900 text-stone-400 border-stone-800 hover:border-stone-600 hover:text-stone-200"}`}>
                  <span>{tab.icon}</span>{tab.label}
                </div>
              ))}
            </div>
          </div>

          <div className="flex items-center gap-3">
            <div className="hidden md:flex items-center gap-2 bg-stone-900 border-2 border-stone-800 rounded-full px-3 py-1.5">
              <div className="h-2 w-2 rounded-full bg-emerald-400 animate-pulse" />
              <span className="text-[11px] font-mono font-bold">NÚCLEO RUST • TIGERBEETLE • CONECTADO</span>
            </div>
            <div className="flex items-center gap-2 bg-amber-400 border-[2.5px] border-black rounded-full px-3 py-1.5 shadow-[3px_3px_0px_black] rotate-[1deg]">
              <span className="text-[11px] font-black text-black">CAJA: RD$ 18,420.00</span>
              <span className="h-5 w-5 bg-black text-amber-400 rounded-full flex items-center justify-center text-[10px] font-black">↗</span>
            </div>
            <div className="h-10 w-10 rounded-full bg-white border-[3px] border-black flex items-center justify-center font-black text-black shadow-[3px_3px_0px_black]">EM</div>
          </div>
        </div>

        {/* Ticker eventos estilo colmado */}
        <div className="bg-black border-y-2 border-stone-800 py-1 overflow-hidden">
          <div className="flex gap-8 animate-[marquee_30s_linear_infinite] whitespace-nowrap text-[10px] font-mono">
            <span className="text-stone-500">EVENTOS LEDGER #1828: VentaCompletada E320000000127 • TB linked 3 transfers atómicos • ETicketAceptado TrackID dgi-982x • QR https://ecf.dgii.gov.do/ • AdelantoSolicitado María RD$2,000 • 50% OK • AdelantoAprobado TB 3001 posted • InventarioReservado ARZ-002 • RFCE 47 facturas &lt;250k enviadas • ARECF recibido proveedor 130000001</span>
          </div>
        </div>
      </header>

      <div className="relative z-10 mx-auto max-w-[1800px] flex min-h-[calc(100vh-78px)]">
        {/* Sidebar izquierdo - Categorías + notas */}
        <aside className="hidden xl:flex w-[300px] flex-col gap-4 p-4 border-r-2 border-stone-800 bg-[#1c1917]/50 backdrop-blur">
          {/* Categorías estilo letrero */}
          <div>
            <h2 className="text-[11px] font-black tracking-[0.2em] text-stone-500 mb-3">CATEGORÍAS • INVENTARIO TIGERBEETLE</h2>
            <div className="space-y-2">
              {[
                { icon: "🌽", nombre: "VÍVERES", count: 42, color: "bg-amber-400", text: "text-black" },
                { icon: "🥤", nombre: "BEBIDAS / FRÍOS", count: 18, color: "bg-sky-400", text: "text-black" },
                { icon: "🍞", nombre: "PANADERÍA", count: 120, color: "bg-orange-300", text: "text-black", alerta: true },
                { icon: "🧴", nombre: "LIMPIEZA", count: 15, color: "bg-violet-300", text: "text-black" },
                { icon: "🥫", nombre: "ENLATADOS", count: 24, color: "bg-emerald-300", text: "text-black" },
                { icon: "🍭", nombre: "DULCES", count: 67, color: "bg-pink-300", text: "text-black" },
              ].map((cat) => (
                <div key={cat.nombre} className="group flex items-center justify-between p-3 rounded-[14px] border-[2.5px] border-stone-800 bg-stone-900 hover:bg-stone-800 hover:border-stone-700 cursor-pointer transition-all hover:rotate-[-0.5deg] hover:shadow-[4px_4px_0px_black]">
                  <div className="flex items-center gap-3">
                    <div className={`h-9 w-9 rounded-[10px] ${cat.color} border-2 border-black flex items-center justify-center text-[16px] shadow-[2px_2px_0px_black]`}>{cat.icon}</div>
                    <div>
                      <p className="font-black text-[12px] tracking-wide">{cat.nombre}</p>
                      <p className="text-[10px] text-stone-500 font-mono">{cat.count} productos • Stock OK</p>
                    </div>
                  </div>
                  {cat.alerta && <span className="h-2 w-2 bg-red-500 rounded-full animate-ping" />}
                </div>
              ))}
            </div>
          </div>

          {/* Nota amarilla adelantos - estilo post-it mano */}
          <div className="relative bg-[#fef08a] border-[3px] border-black rounded-[16px] p-4 shadow-[6px_6px_0px_black] rotate-[-1deg] text-black">
            <div className="absolute -top-2 -right-2 h-6 w-12 bg-black/10 rotate-6 rounded-sm" />
            <h3 className="font-black text-[13px] tracking-tight flex items-center gap-2">💸 ADELANTOS HOY • EWA 50%</h3>
            <p className="text-[10px] font-bold opacity-60 mt-1 leading-tight">No es préstamo. Es sueldo ya ganado. Sin interés. Descuento quincena.</p>
            
            <div className="mt-3 space-y-2.5">
              {[
                { nombre: "María P. • Cajera", ganado: "9,200", disp: "4,600", sol: "2,000", motivo: "Medicina", estado: "✓ Aprobado TB 3001", color: "bg-emerald-400" },
                { nombre: "Juan C. • Almacén", ganado: "6,400", disp: "3,200", sol: "3,000", motivo: "Transporte", estado: "⏳ Pendiente", color: "bg-amber-400" },
              ].map((e) => (
                <div key={e.nombre} className="bg-white border-2 border-black rounded-[10px] p-2.5 shadow-[2px_2px_0px_black]">
                  <p className="font-black text-[11px]">{e.nombre}</p>
                  <p className="text-[9px] font-mono opacity-60">Ganado RD${e.ganado} • Disp 50% RD${e.disp} • {e.motivo}</p>
                  <div className="mt-1.5 flex justify-between items-center">
                    <span className="font-black text-[11px]">RD${e.sol}</span>
                    <span className={`text-[8px] font-black px-1.5 py-0.5 rounded-full border border-black ${e.color}`}>{e.estado}</span>
                  </div>
                </div>
              ))}
            </div>
            
            <p className="text-[8px] font-mono mt-3 opacity-50 leading-tight">Evento: AdelantoAprobado → TB Debe activo:anticipos / Haber caja pending→posted. Deducción nómina “Anticipo Salario”.</p>
          </div>

          {/* Mini ledger - estilo libreta */}
          <div className="rounded-[14px] border-2 border-stone-800 bg-stone-900 p-3">
            <h3 className="font-black text-[11px] tracking-widest text-stone-500">LIBRO MAYOR • TIGERBEETLE VIVO</h3>
            <div className="mt-3 space-y-1.5 font-mono text-[11px]">
              <div className="flex justify-between"><span className="text-stone-500">caja</span><span className="font-bold">RD$ 18,420</span></div>
              <div className="flex justify-between text-amber-300"><span>anticipos</span><span>RD$ 6,000</span></div>
              <div className="flex justify-between text-sky-300"><span>itbis x pagar</span><span>RD$ 4,210</span></div>
              <div className="flex justify-between border-t border-dashed border-stone-700 pt-1.5 mt-1.5"><span className="font-black">ventas</span><span className="font-black text-emerald-400">RD$ 42,100</span></div>
            </div>
          </div>
        </aside>

        {/* Centro - Productos */}
        <main className="flex-1 p-4">
          {/* Buscador */}
          <div className="flex gap-3 mb-4">
            <div className="flex-1 relative">
              <input placeholder="Buscar plátanos, arroz, fríos... (SKU o nombre)" className="w-full bg-black border-[3px] border-stone-800 rounded-full px-5 py-3 text-[13px] font-bold placeholder:text-stone-600 focus:outline-none focus:border-amber-400 focus:shadow-[0_0_0_4px_rgba(251,146,60,0.2)] transition-all" />
              <div className="absolute right-2 top-2 bg-white text-black border-2 border-black rounded-full h-8 w-8 flex items-center justify-center font-black">⌕</div>
            </div>
            <div className="hidden md:flex gap-2">
              {["TODOS", "18% • GRAVADO", "16% • REDUCIDA", "EXENTO"].map((f) => (
                <button key={f} className="px-4 py-2 rounded-full border-[2.5px] border-stone-800 bg-stone-900 text-[11px] font-black tracking-wider hover:bg-white hover:text-black hover:border-black hover:shadow-[3px_3px_0px_black] transition-all">{f}</button>
              ))}
            </div>
          </div>

          {/* Banner DGII */}
          <div className="mb-4 rounded-[14px] border-[3px] border-black bg-sky-400 text-black p-3 flex items-center justify-between shadow-[4px_4px_0px_black]">
            <div className="flex items-center gap-3">
              <div className="h-8 w-8 bg-black text-sky-400 rounded-full flex items-center justify-center font-black">✓</div>
              <div>
                <p className="font-black text-[12px] tracking-tight">CUMPLIMIENTO DGII • e-CF OBLIGATORIO DESDE 15 NOV 2026</p>
                <p className="text-[10px] font-bold opacity-70">E32 &lt; RD$250k → Resumen diario RFCE • E31 con RNC • 606/607 auto desde ledger</p>
              </div>
            </div>
            <span className="text-[10px] font-black bg-black text-sky-400 px-2 py-1 rounded-full">46 ACEPTADAS • 1 PENDIENTE</span>
          </div>

          {/* Grid productos - diseño único con tilt */}
          <div className="grid grid-cols-2 lg:grid-cols-3 gap-4">
            {[
              { sku: "PLT-001", nombre: "PLÁTANOS X LIBRA", precio: "45.00", itbis: "EXENTO", stock: 42, cat: "VÍVERES", emoji: "🍌", bg: "from-yellow-300 to-amber-400", rotate: "-1deg" },
              { sku: "ARZ-002", nombre: "ARROZ PREMIUM 1LB", precio: "118.00", itbis: "18% GRAVADO", stock: 18, cat: "VÍVERES", emoji: "🍚", bg: "from-stone-100 to-stone-300", rotate: "1deg" },
              { sku: "REF-010", nombre: "COCA-COLA 2L • FRÍO", precio: "95.00", itbis: "18% GRAVADO", stock: 3, cat: "FRÍOS", emoji: "🥤", bg: "from-red-500 to-red-700", alerta: true, rotate: "-0.5deg" },
              { sku: "PAN-001", nombre: "PAN SOBAO", precio: "10.00", itbis: "16% REDUCIDA", stock: 120, cat: "PANADERÍA", emoji: "🥖", bg: "from-amber-200 to-orange-300", rotate: "0.8deg" },
              { sku: "ACE-005", nombre: "ACEITE CRISOL 16OZ", precio: "185.00", itbis: "16% REDUCIDA", stock: 24, cat: "VÍVERES", emoji: "🫒", bg: "from-yellow-200 to-lime-300", rotate: "-0.8deg" },
              { sku: "DET-003", nombre: "DETERGENTE ACE", precio: "135.00", itbis: "18% GRAVADO", stock: 15, cat: "LIMPIEZA", emoji: "🧼", bg: "from-blue-300 to-cyan-400", rotate: "1.2deg" },
            ].map((p) => (
              <div key={p.sku} className={`product-card group relative rounded-[20px] border-[3px] border-black bg-gradient-to-br ${p.bg} p-4 shadow-[6px_6px_0px_black] cursor-pointer`} style={{ transform: `rotate(${p.rotate})` }}>
                {p.alerta && <div className="absolute -top-2 -right-2 bg-red-500 text-white border-2 border-black rounded-full px-2 py-0.5 text-[9px] font-black animate-bounce">¡POCO STOCK!</div>}
                <div className="flex justify-between items-start">
                  <span className="bg-black text-white text-[9px] font-black px-2 py-0.5 rounded-full border border-black">{p.cat} • {p.sku}</span>
                  <span className={`h-2 w-2 rounded-full border border-black ${p.stock < 5 ? "bg-red-500" : "bg-emerald-400"}`} />
                </div>
                <div className="mt-2 text-[48px] leading-none drop-shadow-[3px_3px_0px_rgba(0,0,0,0.2)] group-hover:scale-110 transition-transform">{p.emoji}</div>
                <h3 className="mt-2 font-black text-black text-[13px] leading-[0.95] tracking-tight">{p.nombre}</h3>
                <div className="mt-3 flex items-end justify-between">
                  <div>
                    <p className="font-black text-black text-[18px] tracking-tight">RD$ {p.precio}</p>
                    <p className="text-[9px] font-black bg-black text-white inline-block px-1.5 py-0.5 rounded-full mt-1">{p.itbis} • STOCK {p.stock}</p>
                  </div>
                  <div className="h-9 w-9 bg-black text-white rounded-full border-2 border-black flex items-center justify-center font-black text-[18px] shadow-[2px_2px_0px_black] group-hover:rotate-90 transition-transform">+</div>
                </div>
              </div>
            ))}
          </div>

          {/* Stats abajo productos */}
          <div className="mt-6 grid grid-cols-3 gap-3">
            <div className="rounded-[14px] border-[3px] border-black bg-white text-black p-3 shadow-[4px_4px_0px_black]">
              <p className="font-black text-[11px]">VENTAS HOY • 47 E32</p>
              <p className="font-black text-[22px] mt-1">RD$ 18,420</p>
              <p className="text-[10px] font-bold opacity-60">+12% vs ayer • TigerBeetle posted 1,284 events</p>
            </div>
            <div className="rounded-[14px] border-[3px] border-black bg-black text-white p-3 shadow-[4px_4px_0px_white]">
              <p className="font-black text-[11px] text-stone-400">ITBIS COBRADO HOY</p>
              <p className="font-black text-[22px] text-sky-400 mt-1">RD$ 3,211</p>
              <p className="text-[10px] font-bold opacity-60">Para IT-1 • 18% y 16% separado auto</p>
            </div>
            <div className="rounded-[14px] border-[3px] border-amber-400 bg-amber-400 text-black p-3 shadow-[4px_4px_0px_black]">
              <p className="font-black text-[11px]">EVENTOS • LEDGER INMUTABLE</p>
              <p className="font-black text-[22px]">1,284</p>
              <p className="text-[10px] font-bold">Hash encadenado • Auditoría DGII lista</p>
            </div>
          </div>
        </main>

        {/* Derecha - Recibo papel único */}
        <aside className="hidden lg:flex w-[380px] flex-col p-4 gap-4 bg-[#1c1917]/80 backdrop-blur border-l-2 border-stone-800">
          {/* Cliente */}
          <div className="rounded-[14px] border-2 border-stone-800 bg-black p-3 flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="h-10 w-10 rounded-full bg-white border-2 border-black flex items-center justify-center font-black">👤</div>
              <div>
                <p className="font-black text-[12px]">CONSUMIDOR FINAL</p>
                <p className="text-[10px] font-mono text-stone-400">RNC 000000000 • Sin crédito fiscal</p>
              </div>
            </div>
            <span className="text-[10px] font-black bg-stone-800 border border-stone-700 px-2 py-1 rounded-full">E32</span>
          </div>

          {/* Recibo papel */}
          <div className="relative">
            <div className="rounded-t-[16px] bg-[#fafaf9] text-black border-[3px] border-black shadow-[6px_6px_0px_black] overflow-hidden">
              {/* Header recibo */}
              <div className="bg-black text-white p-4 text-center">
                <p className="font-black tracking-[0.2em] text-[11px]">COLMADO EL SOL SRL</p>
                <p className="text-[9px] font-mono opacity-60 mt-1">RNC 130-79375-2 • Av Duarte #123 • SDO • Tel 809-555-0101</p>
                <div className="mt-2 inline-block bg-white text-black font-black text-[10px] px-2 py-0.5 rounded-full">FACTURA CONSUMO ELECTRÓNICA</div>
                <p className="font-mono text-[9px] mt-1 opacity-60">eNCF: E320000000128 • 15-07-2026 21:33 • Caja 01</p>
              </div>

              <div className="p-4">
                <div className="space-y-2.5">
                  <div className="flex justify-between text-[11px] font-bold"><span>PLÁTANOS X 2 LB</span><span>RD$ 90.00</span></div>
                  <div className="flex justify-between text-[11px]"><span className="opacity-60">  45.00 x 2 • EXENTO</span><span>EXENTO</span></div>
                  <div className="flex justify-between text-[11px] font-bold"><span>ARROZ PREMIUM (1)</span><span>RD$ 118.00</span></div>
                  <div className="flex justify-between text-[11px] font-bold"><span>COCA-COLA 2L (1)</span><span>RD$ 95.00</span></div>
                  <div className="border-t-2 border-dashed border-black/20 my-3" />
                  <div className="flex justify-between text-[11px]"><span className="opacity-60">Subtotal Gravado 18%</span><span>RD$ 213.00</span></div>
                  <div className="flex justify-between text-[11px]"><span className="opacity-60">Exento + 16% Reducida</span><span>RD$ 90.00</span></div>
                  <div className="flex justify-between text-[11px]"><span className="opacity-60">ITBIS 18%</span><span>RD$ 38.34</span></div>
                  <div className="flex justify-between font-black text-[16px] border-t-[3px] border-black pt-2 mt-2"><span>TOTAL</span><span>RD$ 341.34</span></div>
                </div>

                <div className="mt-4 bg-black text-white rounded-[10px] p-3">
                  <p className="font-black text-[10px] tracking-widest">PAGO • EFECTIVO</p>
                  <div className="flex justify-between text-[11px] mt-1"><span>Entregado</span><span>RD$ 500.00</span></div>
                  <div className="flex justify-between text-[12px] font-black"><span>Devuelta</span><span className="text-amber-400">RD$ 158.66</span></div>
                </div>

                <div className="mt-4 flex gap-2">
                  <div className="flex-1 border-2 border-black rounded-[10px] p-2 text-center">
                    <div className="h-16 w-16 mx-auto bg-black rounded-[8px] flex items-center justify-center text-white font-black text-[20px]">QR</div>
                    <p className="text-[7px] font-mono mt-1 leading-tight">https://ecf.dgii.gov.do/eCF/ConsultaTimbre?RNCEmisor=13079...&CodigoSeguridad=A1B2C3</p>
                  </div>
                  <div className="flex-1 text-[8px] font-mono leading-tight">
                    <p className="font-black">DGII • TIMBRE ELECTRÓNICO</p>
                    <p className="mt-1 opacity-60">RNC Emisor: 130793752</p>
                    <p className="opacity-60">eNCF: E320000000128</p>
                    <p className="opacity-60">Fecha: 15-07-2026</p>
                    <p className="opacity-60">Total: RD$ 341.34</p>
                    <p className="font-black mt-1">Cod Seg: A1B2C3 • TrackID: dgi-982x</p>
                    <p className="mt-1 bg-amber-400 text-black font-black inline-block px-1 rounded">ACEPTADO</p>
                  </div>
                </div>

                <p className="text-[8px] font-mono text-center mt-4 opacity-40 leading-tight">Conserve este recibo • Evento #1828 • Hash encadenado TigerBeetle • Hecho en RD • Gracias por su compra • Vuelva pronto!</p>
              </div>
            </div>
            {/* Zigzag bottom */}
            <div className="h-4 bg-repeat-x" style={{
              backgroundImage: `radial-gradient(circle at 8px 0px, transparent 8px, #fafaf9 8px)`,
              backgroundSize: '16px 16px',
              backgroundPosition: '0px 0px'
            }} />
          </div>

          <button className="w-full bg-[#facc15] hover:bg-yellow-300 text-black border-[3px] border-black rounded-full py-3.5 font-black text-[13px] tracking-wider shadow-[6px_6px_0px_black] hover:shadow-[8px_8px_0px_black] hover:translate-x-[-2px] hover:translate-y-[-2px] transition-all">
            COBRAR • RD$ 341.34 • GENERAR E32 + QR DGII
          </button>

          <div className="grid grid-cols-3 gap-2">
            <button className="bg-stone-900 border-2 border-stone-800 rounded-full py-2 text-[10px] font-black hover:bg-white hover:text-black hover:border-black transition-colors">EFECTIVO</button>
            <button className="bg-stone-900 border-2 border-stone-800 rounded-full py-2 text-[10px] font-black hover:bg-white hover:text-black hover:border-black transition-colors">TARJETA</button>
            <button className="bg-stone-900 border-2 border-stone-800 rounded-full py-2 text-[10px] font-black hover:bg-white hover:text-black hover:border-black transition-colors">TRANSFER</button>
          </div>

          <p className="text-[9px] font-mono text-stone-500 text-center leading-tight">Flujo bancario: VentaCompletada → Rust Core → TigerBeetle Reserve Inventory linked → XAdES-BES firma → DGII async TrackID → QR impreso</p>
        </aside>
      </div>

      <style>{`@keyframes marquee { 0% { transform: translateX(0); } 100% { transform: translateX(-50%); } }`}</style>
    </div>
  );
}
