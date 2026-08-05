"use client";

import { useEffect, useState } from "react";
import { apiFetch } from "@/lib/api";
import type { Cliente, Empresa } from "./tipos";

export function useEmpresa() {
  const [empresa, setEmpresa] = useState<Empresa | null>(null);
  useEffect(() => {
    apiFetch<Empresa>("/api/config/empresa").then(setEmpresa).catch(() => {});
  }, []);
  return empresa;
}

export function useCliente(clienteId: string | null | undefined) {
  const [cliente, setCliente] = useState<Cliente | null>(null);
  useEffect(() => {
    if (!clienteId) {
      setCliente(null);
      return;
    }
    apiFetch<Cliente>(`/api/clientes/${clienteId}`).then(setCliente).catch(() => setCliente(null));
  }, [clienteId]);
  return cliente;
}
