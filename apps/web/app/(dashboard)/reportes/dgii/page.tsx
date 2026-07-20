export default function Pagina() {
  return (
    <div className="min-h-[calc(100vh-120px)] bg-[#0c0a09] p-6">
      <div className="mx-auto max-w-[1200px]">
        <div className="rounded-[20px] border-[3px] border-black bg-white text-black p-8 shadow-[8px_8px_0px_black]">
          <div className="flex items-center gap-4">
            <div className="h-14 w-14 rounded-[14px] bg-amber-400 border-[3px] border-black flex items-center justify-center text-2xl shadow-[3px_3px_0px_black]">🇩🇴</div>
            <div>
              <h1 className="font-black text-[22px] tracking-tight">DGII • 606/607/608/609/IT-1</h1>
              <p className="text-[12px] font-bold opacity-60 mt-1">Selector periodo AAAAMM, Exportar TXT pre-validado, subir OFV</p>
            </div>
          </div>
          <div className="mt-6 rounded-[14px] border-2 border-dashed border-black/20 bg-amber-50 p-6 text-center">
            <p className="font-black text-[14px]">🚧 MÓDULO EN CONSTRUCCIÓN • PLAN MAESTRO FASE ACTIVA</p>
            <p className="text-[11px] font-mono mt-2 opacity-60">Este módulo está planificado en docs/00-PLAN-MAESTRO-SISTEMA-COMPLETO.md<br/>Entidades, Eventos, API Rust, UI Español, Ledger TigerBeetle, Validaciones DGII</p>
            <div className="mt-4 flex justify-center gap-2">
              <span className="text-[10px] font-black bg-black text-white px-2 py-1 rounded-full">Event Sourcing</span>
              <span className="text-[10px] font-black bg-amber-400 text-black border border-black px-2 py-1 rounded-full">TigerBeetle</span>
              <span className="text-[10px] font-black bg-sky-400 text-black border border-black px-2 py-1 rounded-full">DGII e-CF</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
