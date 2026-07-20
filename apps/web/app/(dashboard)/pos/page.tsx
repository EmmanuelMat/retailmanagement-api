"use client";
import { useState } from "react";

type Producto = {
  sku: string;
  nombre: string;
  precio: number;
  itbis: string;
  stock: number;
  categoria: string;
  emoji: string;
  bg: string;
};

type CarritoItem = Producto & { cantidad: number };

const productos: Producto[] = [
  { sku: "PLT-001", nombre: "PLÁTANOS X LIBRA", precio: 45, itbis: "EXENTO", stock: 42, categoria: "VÍVERES", emoji: "🍌", bg: "from-yellow-300 to-amber-400" },
  { sku: "ARZ-002", nombre: "ARROZ PREMIUM 1LB", precio: 118, itbis: "18% GRAVADO", stock: 18, categoria: "VÍVERES", emoji: "🍚", bg: "from-stone-100 to-stone-300" },
  { sku: "REF-010", nombre: "COCA-COLA 2L • FRÍO", precio: 95, itbis: "18% GRAVADO", stock: 3, categoria: "BEBIDAS", emoji: "🥤", bg: "from-red-500 to-red-700" },
  { sku: "PAN-001", nombre: "PAN SOBAO", precio: 10, itbis: "16% REDUCIDA", stock: 120, categoria: "PANADERÍA", emoji: "🥖", bg: "from-amber-200 to-orange-300" },
  { sku: "PRESIDENTE", nombre: "Presidente 12oz", precio: 150, itbis: "18% GRAVADO", stock: 24, categoria: "BEBIDAS", emoji: "🍺", bg: "from-amber-300 to-yellow-500" },
  { sku: "LECHE-001", nombre: "Leche Rica 1L", precio: 85, itbis: "EXENTO", stock: 15, categoria: "VÍVERES", emoji: "🥛", bg: "from-blue-100 to-blue-300" },
  { sku: "ARROZ-SEL", nombre: "Arroz Selecto 2lb", precio: 72, itbis: "EXENTO", stock: 35, categoria: "VÍVERES", emoji: "🍚", bg: "from-stone-200 to-stone-400" },
  { sku: "MARLBORO", nombre: "Marlboro Red (Ind)", precio: 25, itbis: "18% GRAVADO", stock: 50, categoria: "OTROS", emoji: "🚬", bg: "from-red-400 to-red-600" },
];

const categorias = [
  { nombre: "VÍVERES", count: 42, icon: "🌽", color: "bg-amber-400", activo: true },
  { nombre: "BEBIDAS", count: 102, icon: "🥤", color: "bg-sky-400", activo: false },
  { nombre: "PANADERÍA", count: 15, icon: "🥖", color: "bg-orange-300", activo: false },
  { nombre: "LIMPIEZA", count: 42, icon: "🧴", color: "bg-violet-300", activo: false },
];

export default function POSStitch() {
  const [carrito, setCarrito] = useState<CarritoItem[]>([
    { sku: "PLT-001", nombre: "PLÁTANOS X LIBRA", precio: 45, itbis: "EXENTO", stock: 42, categoria: "VÍVERES", emoji: "🍌", bg: "from-yellow-300 to-amber-400", cantidad: 2 },
    { sku: "ARZ-002", nombre: "Arroz Selecto 2lb", precio: 72, itbis: "EXENTO", stock: 35, categoria: "VÍVERES", emoji: "🍚", bg: "from-stone-200 to-stone-400", cantidad: 2 },
    { sku: "MARLBORO", nombre: "Marlboro Red (Ind)", precio: 25, itbis: "18% GRAVADO", stock: 50, categoria: "OTROS", emoji: "🚬", bg: "from-red-400 to-red-600", cantidad: 1 },
  ]);
  const [categoriaActiva, setCategoriaActiva] = useState("VÍVERES");
  const [busqueda, setBusqueda] = useState("");

  const subtotalGravado = carrito.filter(i => i.itbis.includes("18%")).reduce((s, i) => s + i.precio * i.cantidad, 0);
  const subtotalExento = carrito.filter(i => !i.itbis.includes("18%")).reduce((s, i) => s + i.precio * i.cantidad, 0);
  const itbis = subtotalGravado * 0.18;
  const total = carrito.reduce((s, i) => s + i.precio * i.cantidad, 0) + itbis;
  const entregado = 500;
  const devuelta = entregado - total;

  function agregarProducto(p: Producto) {
    setCarrito(prev => {
      const existe = prev.find(i => i.sku === p.sku);
      if (existe) return prev.map(i => i.sku === p.sku ? { ...i, cantidad: i.cantidad + 1 } : i);
      return [...prev, { ...p, cantidad: 1 }];
    });
  }

  function quitarProducto(sku: string) {
    setCarrito(prev => prev.filter(i => i.sku !== sku));
  }

  const productosFiltrados = productos.filter(p => 
    (categoriaActiva === "VÍVERES" || p.categoria === categoriaActiva) &&
    (busqueda === "" || p.nombre.toLowerCase().includes(busqueda.toLowerCase()) || p.sku.toLowerCase().includes(busqueda.toLowerCase()))
  );

  return (
    <div className="min-h-screen bg-[#151312] text-[#e8e1df] selection:bg-[#facc15] selection:text-black relative overflow-hidden">
      {/* Noise */}
      <div className="pointer-events-none fixed inset-0 opacity-[0.03]" style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E")` }} />

      {/* Ticker */}
      <div className="h-8 bg-black border-b-[3px] border-black flex items-center overflow-hidden sticky top-0 z-[60]">
        <div className="animate-[marquee_30s_linear_infinite] whitespace-nowrap font-mono text-[10px] text-[#4edea3] uppercase px-4 flex gap-8">
          <span>[SYS_OK] LEDGER: 0x882A...7C • NEW TRANSACTION: RD$ 450.00 FROM TERMINAL_01 • STOCK ALERT: CERVEZA 12oz LOW • DGII VALIDATION SUCCESSFUL • EWA LIMIT: 50% ENABLED • RFCE 47 FACTURAS &lt;250K ENVIADAS • ARECF RECIBIDO PROVEEDOR •</span>
          <span>[SYS_OK] LEDGER: 0x882A...7C • NEW TRANSACTION: RD$ 450.00 FROM TERMINAL_01 • STOCK ALERT: CERVEZA 12oz LOW • DGII VALIDATION SUCCESSFUL • EWA LIMIT: 50% ENABLED •</span>
        </div>
      </div>

      {/* Header */}
      <header className="flex justify-between items-center w-full px-6 py-4 sticky top-8 z-50 border-b-[3px] border-black bg-[#151312] h-20">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 bg-[#facc15] border-[3px] border-black shadow-[4px_4px_0px_black] flex items-center justify-center text-2xl">☀️</div>
          <div>
            <h1 className="font-black text-[20px] leading-tight uppercase tracking-tight">Colmado El Sol</h1>
            <p className="font-mono text-[10px] text-[#d1c6ab]">Desde 1998 • RNC 130-79375-2</p>
          </div>
        </div>
        <nav className="hidden md:flex gap-2">
          <button className="bg-[#facc15] text-black border-[3px] border-black shadow-[3px_3px_0px_black] font-black px-5 py-2 text-[11px] tracking-wider">VENTA</button>
          <button className="text-[#e8e1df] font-black px-5 py-2 border-[3px] border-transparent hover:border-black hover:shadow-[3px_3px_0px_black] transition-all text-[11px]">INVENTARIO</button>
          <button className="text-[#e8e1df] font-black px-5 py-2 border-[3px] border-transparent hover:border-black text-[11px]">CLIENTES</button>
          <button className="text-[#e8e1df] font-black px-5 py-2 border-[3px] border-transparent hover:border-black text-[11px]">LIBRO</button>
          <button className="text-[#e8e1df] font-black px-5 py-2 border-[3px] border-transparent hover:border-black text-[11px]">NÓMINA</button>
        </nav>
        <div className="flex items-center gap-3">
          <div className="hidden md:flex items-center gap-2 bg-[#221f1e] border-2 border-black rounded-full px-3 py-1">
            <div className="h-2 w-2 bg-emerald-400 rounded-full animate-pulse" />
            <span className="text-[10px] font-mono font-bold">CAJA RD$ 18,420</span>
          </div>
          <div className="h-9 w-9 bg-white border-[3px] border-black rounded-full flex items-center justify-center font-black text-black">EM</div>
        </div>
      </header>

      <main className="flex h-[calc(100vh-112px)] overflow-hidden">
        {/* Izquierda */}
        <aside className="w-[310px] border-r-[3px] border-black flex flex-col p-4 gap-4 overflow-y-auto bg-[#1d1b1a]">
          <div>
            <h2 className="font-black text-[12px] tracking-[0.2em] text-[#facc15] mb-3">CATEGORÍAS • TIGERBEETLE VIVO</h2>
            <div className="space-y-2.5">
              {categorias.map((cat) => (
                <button
                  key={cat.nombre}
                  onClick={() => setCategoriaActiva(cat.nombre)}
                  className={`w-full flex justify-between items-center border-[3px] border-black p-3 font-black text-[12px] shadow-[4px_4px_0px_black] transition-all hover:translate-x-[-1px] hover:translate-y-[-1px] hover:shadow-[5px_5px_0px_black] ${cat.activo || categoriaActiva === cat.nombre ? "bg-[#facc15] text-black" : "bg-[#221f1e] text-[#e8e1df] hover:bg-[#2c2928]"}`}
                >
                  <span className="flex items-center gap-2">{cat.nombre} {cat.icon}</span>
                  <span className="bg-black text-white text-[10px] px-2 py-0.5 rounded-full border border-black">{cat.count}</span>
                </button>
              ))}
            </div>
          </div>

          <div className="mt-auto bg-[#facc15] p-4 border-[3px] border-black shadow-[6px_6px_0px_black] rotate-[-1.5deg]">
            <div className="flex justify-between items-start">
              <span className="text-[18px]">📌</span>
              <span className="font-mono text-[9px] bg-black text-[#facc15] px-1.5 py-0.5 rounded-full font-black">URGENTE</span>
            </div>
            <p className="font-black text-black text-[14px] leading-none mt-2">ADELANTOS HOY EWA 50%</p>
            <p className="font-mono text-[10px] text-black/60 mt-1">Sistema Ledger: Operacional • 3 activos</p>
            <div className="mt-3 space-y-2">
              <div className="bg-white border-2 border-black rounded-[10px] p-2.5 shadow-[2px_2px_0px_black]">
                <p className="font-black text-[11px] text-black">María P. • Cajera</p>
                <p className="text-[9px] font-mono text-black/60">Ganado RD$9,200 • Disp 50% RD$4,600</p>
                <div className="flex justify-between items-center mt-1">
                  <span className="font-black text-[11px] text-black">RD$2,000 • Medicina</span>
                  <span className="text-[8px] font-black bg-emerald-400 border border-black px-1.5 py-0.5 rounded-full">✓ Aprobado TB 3001</span>
                </div>
              </div>
              <div className="bg-white border-2 border-black rounded-[10px] p-2.5 shadow-[2px_2px_0px_black]">
                <p className="font-black text-[11px] text-black">Juan C. • Almacén</p>
                <p className="text-[9px] font-mono text-black/60">Ganado RD$6,400 • Disp RD$3,200</p>
                <div className="flex justify-between items-center mt-1">
                  <span className="font-black text-[11px] text-black">RD$3,000 • Transporte</span>
                  <span className="text-[8px] font-black bg-amber-400 border border-black px-1.5 py-0.5 rounded-full">⏳ Pendiente</span>
                </div>
              </div>
            </div>
          </div>

          <div className="rounded-[14px] border-2 border-[#373433] bg-[#221f1e] p-3">
            <h3 className="font-black text-[10px] tracking-widest text-[#9a9078]">LIBRO MAYOR • VIVO</h3>
            <div className="mt-3 space-y-1.5 font-mono text-[11px]">
              <div className="flex justify-between"><span className="text-[#9a9078]">caja</span><span className="font-bold text-[#e8e1df]">RD$ 18,420</span></div>
              <div className="flex justify-between text-[#facc15]"><span>anticipos</span><span>RD$ 6,000</span></div>
              <div className="flex justify-between text-[#7bd0ff]"><span>itbis x pagar</span><span>RD$ 4,210</span></div>
              <div className="flex justify-between border-t border-dashed border-[#373433] pt-1.5 mt-1.5"><span className="font-black">ventas</span><span className="font-black text-[#4edea3]">RD$ 42,100</span></div>
            </div>
          </div>
        </aside>

        {/* Centro */}
        <section className="flex-1 p-5 overflow-y-auto bg-[#151312]">
          <div className="max-w-5xl mx-auto space-y-5">
            <div className="flex gap-3">
              <div className="relative flex-1">
                <span className="absolute left-4 top-1/2 -translate-y-1/2 text-[#9a9078]">⌕</span>
                <input
                  value={busqueda}
                  onChange={(e) => setBusqueda(e.target.value)}
                  placeholder="Buscar plátanos, arroz, fríos... (SKU o nombre)"
                  className="w-full bg-[#221f1e] border-[3px] border-black shadow-[4px_4px_0px_black] rounded-full py-3 pl-12 pr-4 font-bold text-[13px] text-white placeholder:text-stone-500 focus:outline-none focus:border-[#facc15] focus:shadow-[0_0_0_4px_rgba(250,204,21,0.2)] transition-all"
                />
              </div>
              <div className="hidden md:flex bg-[#99d9ff] text-[#001e2c] border-[3px] border-black shadow-[4px_4px_0px_black] items-center px-4 gap-2 font-black text-[11px] rounded-full">
                <span>✓</span> CUMPLIMIENTO DGII e-CF obligatorio
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {productosFiltrados.map((p) => (
                <div
                  key={p.sku}
                  onClick={() => agregarProducto(p)}
                  className={`group cursor-pointer relative rounded-[20px] border-[3px] border-black p-4 shadow-[6px_6px_0px_black] transition-all hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[8px_8px_0px_black] hover:rotate-[-1deg] bg-gradient-to-br ${p.bg}`}
                >
                  {p.stock < 5 && <div className="absolute -top-2 -right-2 bg-red-500 text-white border-2 border-black rounded-full px-2 py-0.5 text-[9px] font-black animate-bounce">¡POCO STOCK!</div>}
                  <div className="flex justify-between items-start">
                    <span className="bg-black text-[#facc15] text-[9px] font-black px-2 py-0.5 rounded-full border border-black">{p.sku} • {p.categoria}</span>
                    <span className={`h-2.5 w-2.5 rounded-full border-2 border-black ${p.stock < 5 ? "bg-red-500" : "bg-emerald-400"}`} />
                  </div>
                  <div className="text-[52px] leading-none mt-3 drop-shadow-[3px_3px_0px_rgba(0,0,0,0.3)] group-hover:scale-110 transition-transform">{p.emoji}</div>
                  <h3 className="mt-3 font-black text-black text-[13px] leading-[0.9] tracking-tight">{p.nombre}</h3>
                  <div className="mt-3 flex items-end justify-between">
                    <div>
                      <p className="font-black text-black text-[18px]">RD$ {p.precio}.00</p>
                      <p className="text-[8px] font-black bg-black text-white inline-block px-1.5 py-0.5 rounded-full mt-1">{p.itbis} • STOCK {p.stock}</p>
                    </div>
                    <div className="h-10 w-10 bg-black text-[#facc15] rounded-full border-2 border-black flex items-center justify-center font-black text-[20px] shadow-[2px_2px_0px_black] group-hover:rotate-90 transition-transform">+</div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Derecha - Recibo */}
        <aside className="w-[400px] border-l-[3px] border-black flex flex-col bg-[#1d1b1a] relative">
          <div className="flex-1 overflow-hidden p-4">
            <div className="bg-[#fafaf9] text-black border-[3px] border-black shadow-[6px_6px_0px_black] rounded-t-[16px] overflow-hidden flex flex-col h-full">
              <div className="bg-black text-white p-4 text-center">
                <p className="font-black tracking-[0.2em] text-[11px]">REGISTRO #0942 • COLMADO EL SOL</p>
                <p className="font-mono text-[9px] opacity-60 mt-1">RNC 130-79375-2 • Av Duarte #123 • 15-07-2026 21:33 • Caja 01</p>
                <div className="mt-2 inline-block bg-white text-black font-black text-[9px] px-2 py-0.5 rounded-full">FACTURA CONSUMO ELECTRÓNICA E32</div>
                <p className="font-mono text-[8px] mt-1 opacity-60">eNCF: E320000000128 • CONSUMIDOR FINAL 000000000</p>
              </div>

              <div className="flex-1 p-5 font-mono text-[11px] overflow-y-auto">
                <div className="space-y-2.5">
                  {carrito.map((item) => (
                    <div key={item.sku} className="group flex justify-between border-b border-black/10 pb-2 hover:bg-amber-50 cursor-pointer" onClick={() => quitarProducto(item.sku)}>
                      <span className="group-hover:line-through">{item.cantidad}x {item.nombre}</span>
                      <span className="font-bold">RD$ {item.precio * item.cantidad}.00</span>
                    </div>
                  ))}
                  {carrito.length === 0 && <p className="text-center opacity-30 py-8">Carrito vacío • Agrega productos</p>}
                </div>

                <div className="mt-6 space-y-1.5 text-[11px]">
                  <div className="flex justify-between opacity-60"><span>Subtotal Gravado 18%</span><span>RD$ {subtotalGravado.toFixed(2)}</span></div>
                  <div className="flex justify-between opacity-60"><span>Exento + 16% Reducida</span><span>RD$ {subtotalExento.toFixed(2)}</span></div>
                  <div className="flex justify-between opacity-60"><span>ITBIS 18%</span><span>RD$ {itbis.toFixed(2)}</span></div>
                  <div className="flex justify-between font-black text-[16px] border-t-[3px] border-black border-dashed pt-3 mt-3">
                    <span>TOTAL</span><span>RD$ {total.toFixed(2)}</span>
                  </div>
                </div>

                <div className="mt-6 space-y-3">
                  <p className="font-black text-[10px] tracking-widest">MÉTODO DE PAGO</p>
                  <div className="grid grid-cols-3 gap-2">
                    <button className="border-2 border-black p-2 bg-black text-white font-black text-[10px] rounded-[8px]">EFECTIVO</button>
                    <button className="border-2 border-black p-2 hover:bg-black/5 font-black text-[10px] rounded-[8px]">TARJETA</button>
                    <button className="border-2 border-black p-2 hover:bg-black/5 font-black text-[10px] rounded-[8px]">CRÉDITO</button>
                  </div>
                  <div className="bg-black/5 p-3 border-2 border-black border-dotted rounded-[10px]">
                    <p className="text-[11px]">Entrega: RD$ {entregado.toFixed(2)}</p>
                    <p className="font-black text-[16px]">Cambio: <span className="text-emerald-600">RD$ {devuelta > 0 ? devuelta.toFixed(2) : "0.00"}</span></p>
                  </div>
                </div>

                <div className="mt-6 flex gap-3 opacity-80">
                  <div className="w-20 h-20 bg-black rounded-[8px] flex items-center justify-center text-white font-black text-[24px] border-2 border-black">QR</div>
                  <div className="flex-1 text-[8px] leading-tight font-mono">
                    <p className="font-black">DGII • TIMBRE ELECTRÓNICO</p>
                    <p className="mt-1">RNC Emisor: 130793752</p>
                    <p>eNCF: E320000000128</p>
                    <p>Fecha: 15-07-2026</p>
                    <p>Total: RD$ {total.toFixed(2)}</p>
                    <p className="font-black mt-1">Cod Seg: A1B2C3 • TrackID: dgi-982x</p>
                    <p className="mt-1 bg-amber-400 text-black font-black inline-block px-1 rounded">ACEPTADO DGII</p>
                  </div>
                </div>

                <p className="text-[8px] font-mono text-center mt-6 opacity-30 leading-tight">
                  Conserve este recibo • Evento #1828 • Hash encadenado TigerBeetle • Hecho en RD • Gracias por su compra • Vuelva pronto!
                </p>
              </div>
            </div>
            <div className="h-4 bg-repeat-x -mt-[1px]" style={{ backgroundImage: `radial-gradient(circle at 8px 0px, transparent 8px, #fafaf9 8px)`, backgroundSize: '16px 16px' }} />
          </div>

          <div className="p-4 bg-black">
            <button className="w-full bg-[#facc15] text-black border-[3px] border-black shadow-[4px_4px_0px_black] py-4 font-black text-[14px] tracking-wider uppercase hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[6px_6px_0px_black] transition-all flex items-center justify-center gap-3">
              <span className="text-[20px]">💳</span> COBRAR RD$ {total.toFixed(2)} • E32 + QR DGII
            </button>
            <p className="text-[9px] font-mono text-stone-500 text-center mt-2 leading-tight">Flujo bancario: VentaCompletada → Rust Core → TigerBeetle Reserve Inventory linked → XAdES-BES firma → DGII async TrackID → QR impreso • 10 años retención</p>
          </div>
        </aside>
      </main>

      <footer className="fixed bottom-0 w-full h-[28px] flex justify-between items-center px-4 z-50 bg-black text-white font-mono text-[10px] border-t-[3px] border-black">
        <div className="flex gap-4">
          <span>DGII Compliant v2.4 | System Ledger: 0x882A...7C | EventStore: 1,284 events hash-encadenados</span>
          <span className="text-[#4edea3]">Terminal 01: ACTIVE • Caja Abierta • RD$ 18,420</span>
        </div>
        <div className="flex gap-4 uppercase">
          <span>Hecho en 🇩🇴 SDO • Rust + TigerBeetle • Español 100%</span>
          <button className="text-[#facc15] font-black hover:underline">Cerrar Caja</button>
        </div>
      </footer>

      <style>{`@keyframes marquee { 0% { transform: translateX(0%); } 100% { transform: translateX(-50%); } }`}</style>
    </div>
  );
}
