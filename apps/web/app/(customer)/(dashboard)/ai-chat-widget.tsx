"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Bot, Send, User, CheckCircle2, X, Sparkles } from "lucide-react";
import { Button, Card, CardContent, Input } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface AccionPropuesta {
  entidad: string;
  campos: Record<string, unknown>;
}

interface Mensaje {
  de: "usuario" | "asistente";
  texto?: string;
  accion?: AccionPropuesta;
  confirmada?: boolean;
}

// Mapea cada entidad que la IA puede proponer a la página real donde se crea
// - la IA nunca crea nada directamente, solo pre-llena este formulario ya
// existente (mismo endpoint, mismo role_guard, misma validación de siempre).
const DESTINOS: Record<string, { ruta: string; label: string }> = {
  producto: { ruta: "/inventario/productos/nuevo", label: "producto" },
  cliente: { ruta: "/clientes/nuevo", label: "cliente" },
  proveedor: { ruta: "/proveedores/nuevo", label: "proveedor" },
  gasto: { ruta: "/gastos", label: "gasto" },
};

const SALUDO: Mensaje = {
  de: "asistente",
  texto: "Hola, soy el asistente de Colmado POS. Puedo responder preguntas o ayudarte a crear un cliente, producto, proveedor o gasto según tu rol. ¿En qué te ayudo?",
};

// Montado una sola vez en el layout del dashboard - flota sobre cualquier
// página y conserva la conversación al navegar, en vez de ser una página
// aparte que hay que abandonar para volver al trabajo.
export default function AiChatWidget() {
  const router = useRouter();
  const [abierto, setAbierto] = useState(false);
  const [mensajes, setMensajes] = useState<Mensaje[]>([SALUDO]);
  const [input, setInput] = useState("");
  const [enviando, setEnviando] = useState(false);

  async function enviar() {
    const texto = input.trim();
    if (!texto || enviando) return;
    setInput("");
    setMensajes((m) => [...m, { de: "usuario", texto }]);
    setEnviando(true);
    try {
      const res = await apiFetch<{ tipo: string; texto?: string; entidad?: string; campos?: Record<string, unknown> }>("/api/ai/chat", {
        method: "POST",
        body: JSON.stringify({ mensaje: texto }),
      });
      if (res.tipo === "accion_propuesta" && res.entidad && res.campos) {
        setMensajes((m) => [...m, { de: "asistente", accion: { entidad: res.entidad!, campos: res.campos! } }]);
      } else {
        setMensajes((m) => [...m, { de: "asistente", texto: res.texto || "..." }]);
      }
    } catch (e: any) {
      setMensajes((m) => [...m, { de: "asistente", texto: "El asistente no está disponible en este momento." }]);
    } finally {
      setEnviando(false);
    }
  }

  function confirmarAccion(idx: number, accion: AccionPropuesta) {
    const destino = DESTINOS[accion.entidad];
    if (!destino) return;
    sessionStorage.setItem(`ai_prefill_${accion.entidad}`, JSON.stringify(accion.campos));
    setMensajes((m) => m.map((msg, i) => (i === idx ? { ...msg, confirmada: true } : msg)));
    router.push(destino.ruta as any);
    setAbierto(false);
  }

  if (!abierto) {
    return (
      <button
        onClick={() => setAbierto(true)}
        className="fixed bottom-5 right-5 z-50 h-14 w-14 rounded-full bg-primary text-primary-foreground shadow-lg hover:shadow-xl hover:scale-105 transition-all flex items-center justify-center"
        aria-label="Abrir asistente"
      >
        <Sparkles className="h-6 w-6" />
      </button>
    );
  }

  return (
    <Card className="fixed bottom-5 right-5 z-50 w-[380px] h-[560px] max-h-[75vh] flex flex-col shadow-xl">
      <div className="flex items-center justify-between px-4 h-14 border-b border-border shrink-0">
        <div className="flex items-center gap-2">
          <div className="h-7 w-7 rounded-full bg-primary/10 text-primary flex items-center justify-center">
            <Bot className="h-4 w-4" />
          </div>
          <span className="font-semibold text-sm">Asistente</span>
        </div>
        <button onClick={() => setAbierto(false)} aria-label="Cerrar" className="text-muted-foreground hover:text-foreground">
          <X className="h-4 w-4" />
        </button>
      </div>

      <CardContent className="flex-1 overflow-y-auto space-y-3 py-4 min-h-0">
        {mensajes.map((msg, i) => (
          <div key={i} className={`flex gap-2 ${msg.de === "usuario" ? "justify-end" : "justify-start"}`}>
            {msg.de === "asistente" && (
              <div className="h-6 w-6 rounded-full bg-primary/10 text-primary flex items-center justify-center shrink-0 mt-0.5">
                <Bot className="h-3.5 w-3.5" />
              </div>
            )}
            <div className={`max-w-[80%] ${msg.de === "usuario" ? "order-first" : ""}`}>
              {msg.accion ? (
                <Card className="border-primary/25">
                  <CardContent className="pt-3 space-y-2 px-3 pb-3">
                    <p className="text-xs font-medium">
                      Puedo crear este {DESTINOS[msg.accion.entidad]?.label || msg.accion.entidad}:
                    </p>
                    <div className="rounded-md bg-muted/50 p-2 text-[11px] space-y-1">
                      {Object.entries(msg.accion.campos).map(([k, v]) => (
                        <div key={k} className="flex justify-between gap-3">
                          <span className="text-muted-foreground">{k}</span>
                          <span className="font-medium text-right truncate">{String(v)}</span>
                        </div>
                      ))}
                    </div>
                    {msg.confirmada ? (
                      <p className="text-[11px] text-success flex items-center gap-1.5"><CheckCircle2 className="h-3 w-3" />Abriendo el formulario...</p>
                    ) : (
                      <Button size="sm" onClick={() => confirmarAccion(i, msg.accion!)}>Revisar y confirmar</Button>
                    )}
                  </CardContent>
                </Card>
              ) : (
                <div className={`rounded-lg px-3 py-2 text-xs leading-relaxed ${msg.de === "usuario" ? "bg-primary text-primary-foreground" : "bg-muted"}`}>
                  {msg.texto}
                </div>
              )}
            </div>
            {msg.de === "usuario" && (
              <div className="h-6 w-6 rounded-full bg-muted flex items-center justify-center shrink-0 mt-0.5">
                <User className="h-3 w-3" />
              </div>
            )}
          </div>
        ))}
        {enviando && (
          <div className="flex gap-2 justify-start">
            <div className="h-6 w-6 rounded-full bg-primary/10 text-primary flex items-center justify-center shrink-0">
              <Bot className="h-3.5 w-3.5" />
            </div>
            <div className="rounded-lg px-3 py-2 text-xs bg-muted text-muted-foreground">Pensando...</div>
          </div>
        )}
      </CardContent>

      <div className="flex gap-2 p-3 border-t border-border shrink-0">
        <Input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && enviar()}
          placeholder="Escribe tu mensaje..."
          disabled={enviando}
          className="h-9 text-sm"
        />
        <Button onClick={enviar} disabled={enviando || !input.trim()} size="icon" className="h-9 w-9 shrink-0">
          <Send className="h-4 w-4" />
        </Button>
      </div>
    </Card>
  );
}
