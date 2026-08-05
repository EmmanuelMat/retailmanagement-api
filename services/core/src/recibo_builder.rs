//! Constructor de tickets ESC/POS para impresoras térmicas de red (80mm/58mm).
//! El layout sigue el formato estándar de una factura de consumo electrónica
//! dominicana: encabezado del negocio, caja de e-NCF/vencimiento, datos del
//! cliente, renglones, totales (Gravado/Exento/ITBIS) y el bloque de firma
//! electrónica (QR + código de seguridad) que exige la DGII en cada e-CF.

use rust_decimal::Decimal;

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;

pub struct ReciboEmisor {
    pub razon_social: String,
    pub rnc: String,
    pub direccion: String,
    pub telefono: Option<String>,
}

pub struct ReciboCliente {
    pub nombre: String,
    pub rnc_cedula: Option<String>,
}

pub struct ReciboLinea {
    pub nombre: String,
    pub cantidad: Decimal,
    pub precio_unitario: Decimal,
    pub subtotal: Decimal,
}

pub struct ReciboFiscal {
    pub e_ncf: String,
    pub tipo_ecf: i32,
    pub fecha_vencimiento_secuencia: String,
    pub codigo_seguridad: String,
    pub qr_url: String,
    pub fecha_firma: String,
}

pub struct ReciboVenta {
    pub emisor: ReciboEmisor,
    pub cliente: ReciboCliente,
    pub items: Vec<ReciboLinea>,
    pub subtotal: Decimal,
    pub itbis_total: Decimal,
    pub total: Decimal,
    pub metodo_pago: String,
    pub fecha_emision: String, // DD/MM/YYYY HH:MM
    pub fiscal: Option<ReciboFiscal>,
}

fn tipo_ecf_label(tipo: i32) -> &'static str {
    match tipo {
        31 => "FACTURA DE CREDITO FISCAL ELECTRONICA",
        32 => "FACTURA DE CONSUMO ELECTRONICA",
        33 => "NOTA DE DEBITO ELECTRONICA",
        34 => "NOTA DE CREDITO ELECTRONICA",
        _ => "COMPROBANTE FISCAL ELECTRONICO",
    }
}

/// Ancho en columnas de texto (Font A) según el papel configurado.
fn columnas(ancho_mm: i32) -> usize {
    if ancho_mm <= 58 { 32 } else { 48 }
}

fn center(cols: usize, s: &str) -> String {
    let len = s.chars().count();
    if len >= cols {
        return s.chars().take(cols).collect();
    }
    let pad = (cols - len) / 2;
    format!("{}{}", " ".repeat(pad), s)
}

fn rule(cols: usize) -> String {
    "-".repeat(cols)
}

/// Alinea una etiqueta a la izquierda y un valor a la derecha en una sola línea.
fn kv_line(cols: usize, label: &str, value: &str) -> String {
    let total = label.chars().count() + value.chars().count();
    if total >= cols {
        return format!("{}{}", label, value);
    }
    format!("{}{}{}", label, " ".repeat(cols - total), value)
}

fn money(d: Decimal) -> String {
    format!("{:.2}", d)
}

struct Writer {
    buf: Vec<u8>,
    cols: usize,
}

impl Writer {
    fn new(cols: usize) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[ESC, b'@']); // init
        Self { buf, cols }
    }

    fn align(&mut self, n: u8) {
        self.buf.extend_from_slice(&[ESC, b'a', n]);
    }

    fn bold(&mut self, on: bool) {
        self.buf.extend_from_slice(&[ESC, b'E', if on { 1 } else { 0 }]);
    }

    fn size(&mut self, w: u8, h: u8) {
        let n = ((w.saturating_sub(1)) << 4) | h.saturating_sub(1);
        self.buf.extend_from_slice(&[GS, b'!', n]);
    }

    fn text(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(b'\n');
    }

    fn raw_line(&mut self, s: &str) {
        self.text(s);
    }

    fn feed(&mut self, n: u8) {
        self.buf.extend_from_slice(&[ESC, b'd', n]);
    }

    fn rule(&mut self) {
        let cols = self.cols;
        self.raw_line(&rule(cols));
    }

    /// Imprime un código QR usando el juego de comandos "GS ( k" (función 165/167/169/180/181),
    /// soportado por la inmensa mayoría de impresoras térmicas de red compatibles ESC/POS.
    fn qr(&mut self, data: &str) {
        let bytes = data.as_bytes();
        // Modelo 2
        self.buf.extend_from_slice(&[GS, b'(', b'k', 0x04, 0x00, 0x31, 0x41, 0x32, 0x00]);
        // Tamaño de módulo (1-16)
        self.buf.extend_from_slice(&[GS, b'(', b'k', 0x03, 0x00, 0x31, 0x43, 0x06]);
        // Nivel de corrección de errores: M (15%)
        self.buf.extend_from_slice(&[GS, b'(', b'k', 0x03, 0x00, 0x31, 0x45, 0x31]);
        // Almacenar datos
        let store_len = bytes.len() + 3;
        let pl = (store_len & 0xFF) as u8;
        let ph = ((store_len >> 8) & 0xFF) as u8;
        self.buf.extend_from_slice(&[GS, b'(', b'k', pl, ph, 0x31, 0x50, 0x30]);
        self.buf.extend_from_slice(bytes);
        // Imprimir
        self.buf.extend_from_slice(&[GS, b'(', b'k', 0x03, 0x00, 0x31, 0x51, 0x30]);
    }

    fn cut(&mut self) {
        self.feed(3);
        self.buf.extend_from_slice(&[GS, b'V', 0x00]);
    }
}

/// Construye los bytes ESC/POS completos del ticket de una venta, listos para
/// enviar por TCP crudo (puerto 9100) a la impresora de red configurada.
pub fn build_recibo_escpos(venta: &ReciboVenta, ancho_mm: i32, copias: i32) -> Vec<u8> {
    let cols = columnas(ancho_mm);
    let mut w = Writer::new(cols);

    for _ in 0..copias.max(1) {
        // --- Encabezado del negocio ---
        w.align(1);
        w.bold(true);
        w.size(2, 2);
        w.raw_line(&venta.emisor.razon_social);
        w.size(1, 1);
        w.bold(false);
        w.raw_line(&format!("RNC: {}", venta.emisor.rnc));
        w.raw_line(&venta.emisor.direccion);
        if let Some(tel) = &venta.emisor.telefono {
            if !tel.trim().is_empty() {
                w.raw_line(&format!("Tel: {}", tel));
            }
        }
        w.rule();

        // --- Caja fiscal: tipo de documento, NCF, vencimiento ---
        w.align(0);
        if let Some(f) = &venta.fiscal {
            w.bold(true);
            w.raw_line(&center(cols, tipo_ecf_label(f.tipo_ecf)));
            w.bold(false);
            w.raw_line(&kv_line(cols, "NCF:", &f.e_ncf));
            w.raw_line(&kv_line(cols, "Fecha:", &venta.fecha_emision));
            w.raw_line(&kv_line(cols, "Vence secuencia:", &f.fecha_vencimiento_secuencia));
        } else {
            w.bold(true);
            w.raw_line(&center(cols, "TICKET DE VENTA (SIN e-CF)"));
            w.bold(false);
            w.raw_line(&kv_line(cols, "Fecha:", &venta.fecha_emision));
        }
        w.raw_line(&kv_line(cols, "Metodo de pago:", &venta.metodo_pago));
        w.rule();

        // --- Cliente ---
        w.raw_line(&format!("Cliente: {}", venta.cliente.nombre));
        if let Some(rnc) = &venta.cliente.rnc_cedula {
            if !rnc.trim().is_empty() {
                w.raw_line(&format!("RNC/Cedula: {}", rnc));
            }
        }
        w.rule();

        // --- Renglones ---
        for item in &venta.items {
            w.raw_line(&item.nombre);
            let detalle = format!("{} x {}", item.cantidad, money(item.precio_unitario));
            w.raw_line(&kv_line(cols, &detalle, &money(item.subtotal)));
        }
        w.rule();

        // --- Totales ---
        w.raw_line(&kv_line(cols, "Subtotal:", &money(venta.subtotal)));
        w.raw_line(&kv_line(cols, "ITBIS:", &money(venta.itbis_total)));
        w.bold(true);
        w.size(1, 2);
        w.raw_line(&kv_line(cols, "TOTAL RD$:", &money(venta.total)));
        w.size(1, 1);
        w.bold(false);
        w.rule();

        // --- Firma electrónica DGII (QR + código de seguridad) ---
        if let Some(f) = &venta.fiscal {
            w.align(1);
            w.qr(&f.qr_url);
            w.feed(1);
            w.align(0);
            w.raw_line(&format!("Codigo de Seguridad: {}", f.codigo_seguridad));
            w.raw_line(&format!("Firma electronica: {}", f.fecha_firma));
            w.rule();
        }

        w.align(1);
        w.raw_line("Gracias por su compra!");
        w.feed(1);
        w.cut();
    }

    w.buf
}
