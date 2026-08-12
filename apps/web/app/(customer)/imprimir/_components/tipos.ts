export interface Empresa {
  rnc: string;
  razon_social: string;
  nombre_comercial: string | null;
  direccion: string;
  telefono: string | null;
  correo: string | null;
  logo_url: string | null;
}

export interface Cliente {
  id: string;
  nombre: string;
  rnc_cedula: string | null;
  telefono: string | null;
  direccion: string | null;
}
