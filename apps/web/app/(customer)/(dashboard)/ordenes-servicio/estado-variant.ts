export const ESTADO_VARIANT: Record<string, "default" | "success" | "warning" | "destructive" | "secondary"> = {
  BORRADOR: "secondary",
  PROGRAMADA: "default",
  EN_PROCESO: "warning",
  PAUSADA: "secondary",
  COMPLETADA: "success",
  CANCELADA: "destructive",
};
