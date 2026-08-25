export const ESTADO_VARIANT: Record<string, "default" | "success" | "warning" | "destructive" | "secondary"> = {
  BORRADOR: "secondary",
  ENVIADA: "default",
  RECIBIDA_PARCIAL: "warning",
  RECIBIDA: "success",
  CANCELADA: "destructive",
};
