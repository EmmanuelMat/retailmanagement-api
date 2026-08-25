//! Resumen diario con IA - un LLM local pequeño (Ollama, ver docker-compose.yml)
//! convierte números que YA calculamos en Rust en un párrafo amistoso en
//! español. El modelo nunca calcula ni inventa una cifra - solo redacta las
//! que le pasamos, así que no puede alucinar un total o un producto que no
//! existe. Si Ollama no responde o tarda demasiado, se cae a un mensaje
//! armado con `format!` (sin IA) en vez de fallar o colgar el dashboard.

use chrono::{DateTime, Datelike, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProductoBajoStock {
    pub nombre: String,
    pub stock_actual: Decimal,
    pub stock_minimo: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct DigestResult {
    pub mensaje: String,
    pub generado_por_ia: bool,
    pub productos_bajo_minimo: Vec<ProductoBajoStock>,
    pub ventas_hoy_total: Decimal,
    pub caja_abierta: bool,
    pub generado_at: DateTime<Utc>,
}

pub struct AiService {
    pool: PgPool,
    ollama_url: String,
    model: String,
    http: reqwest::Client,
    cache: Mutex<HashMap<String, (DigestResult, Instant)>>,
}

impl AiService {
    pub fn new(pool: PgPool, ollama_url: String, model: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(25))
            .build()
            .expect("No se pudo construir el cliente HTTP para Ollama");
        Self { pool, ollama_url, model, http, cache: Mutex::new(HashMap::new()) }
    }

    /// Infalible a propósito - cualquier fallo (Ollama caído, lento, mal
    /// formado) se resuelve con el mensaje de respaldo, nunca con un error
    /// que el caller tenga que manejar.
    pub async fn digest_diario(&self, tenant_id: &str) -> DigestResult {
        if let Some((cached, at)) = self.cache.lock().unwrap().get(tenant_id) {
            if at.elapsed() < CACHE_TTL {
                return cached.clone();
            }
        }

        let hechos = match self.recolectar_hechos(tenant_id).await {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("No se pudieron recolectar los hechos del resumen para {}: {}", tenant_id, e);
                // Sin datos no hay nada que redactar - un resumen vacío pero honesto.
                (Vec::new(), Decimal::ZERO, false)
            }
        };
        let (productos_bajo_minimo, ventas_hoy_total, caja_abierta) = hechos;

        let hechos_texto = Self::formatear_hechos(&productos_bajo_minimo, ventas_hoy_total, caja_abierta);
        let (mensaje, generado_por_ia) = match self.pedir_a_ollama(&hechos_texto).await {
            Ok(texto) => (texto, true),
            Err(e) => {
                tracing::warn!("Ollama no disponible para el resumen de {}, usando plantilla simple: {}", tenant_id, e);
                (Self::mensaje_plantilla(&productos_bajo_minimo, ventas_hoy_total, caja_abierta), false)
            }
        };

        let result = DigestResult {
            mensaje,
            generado_por_ia,
            productos_bajo_minimo,
            ventas_hoy_total,
            caja_abierta,
            generado_at: Utc::now(),
        };
        self.cache.lock().unwrap().insert(tenant_id.to_string(), (result.clone(), Instant::now()));
        result
    }

    async fn recolectar_hechos(&self, tenant_id: &str) -> anyhow::Result<(Vec<ProductoBajoStock>, Decimal, bool)> {
        let productos_bajo_minimo = sqlx::query_as::<_, ProductoBajoStock>(
            r#"SELECT nombre, stock_actual, stock_minimo FROM productos
               WHERE tenant_id = $1 AND activo = true AND tipo = 'PRODUCTO' AND stock_actual <= stock_minimo
               ORDER BY (stock_actual - stock_minimo) ASC LIMIT 5"#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let ventas_hoy_total: Option<Decimal> = sqlx::query_scalar(
            "SELECT SUM(total) FROM ventas WHERE tenant_id = $1 AND created_at::date = CURRENT_DATE",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        let caja_abierta: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM caja_sesiones WHERE tenant_id = $1 AND estado = 'ABIERTA')",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((productos_bajo_minimo, ventas_hoy_total.unwrap_or_default(), caja_abierta))
    }

    fn formatear_hechos(productos: &[ProductoBajoStock], ventas_hoy: Decimal, caja_abierta: bool) -> String {
        let stock = if productos.is_empty() {
            "Ningún producto está bajo su mínimo de stock.".to_string()
        } else {
            let lista: Vec<String> = productos.iter()
                .map(|p| format!("{} ({} de {})", p.nombre, p.stock_actual, p.stock_minimo))
                .collect();
            format!("Productos con stock bajo: {}.", lista.join(", "))
        };
        format!(
            "Ventas de hoy: RD${}. Caja: {}. {}",
            ventas_hoy,
            if caja_abierta { "abierta" } else { "cerrada" },
            stock,
        )
    }

    fn mensaje_plantilla(productos: &[ProductoBajoStock], ventas_hoy: Decimal, caja_abierta: bool) -> String {
        let mut partes = vec![format!("Hoy llevas RD${} en ventas.", ventas_hoy)];
        if !caja_abierta {
            partes.push("La caja está cerrada.".to_string());
        }
        if !productos.is_empty() {
            let nombres: Vec<&str> = productos.iter().map(|p| p.nombre.as_str()).collect();
            partes.push(format!("Se te está acabando: {}.", nombres.join(", ")));
        }
        partes.join(" ")
    }

    async fn pedir_a_ollama(&self, hechos: &str) -> anyhow::Result<String> {
        let prompt = format!(
            "Convierte estos datos de un colmado en la República Dominicana en un párrafo breve \
             (2-3 frases), amistoso y en español dominicano, para el dueño del negocio. No inventes \
             cifras ni productos que no estén en los datos - usa exactamente los que te doy.\n\nDatos: {}",
            hechos
        );
        self.generar(&prompt).await
    }

    /// Llamada cruda a Ollama con un prompt ya armado - usada tanto por el
    /// resumen diario como por el chat, cada uno con su propio prompt.
    async fn generar(&self, prompt: &str) -> anyhow::Result<String> {
        let res = self.http.post(format!("{}/api/generate", self.ollama_url))
            .json(&serde_json::json!({ "model": self.model, "prompt": prompt, "stream": false }))
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("Ollama respondió {}", res.status());
        }

        let body: serde_json::Value = res.json().await?;
        let texto = body.get("response").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Respuesta de Ollama sin campo 'response'"))?
            .trim()
            .to_string();
        if texto.is_empty() {
            anyhow::bail!("Ollama devolvió una respuesta vacía");
        }
        Ok(texto)
    }

    /// Único mensaje del usuario, sin historial persistido - mantiene el
    /// primer corte simple; cada llamada es una petición independiente.
    pub async fn chat(&self, tenant_id: &str, rol: &str, mensaje: &str) -> ChatResponse {
        let acciones = acciones_para_rol(rol);
        let herramientas = herramientas_para_rol(rol);

        let lista_acciones = if acciones.is_empty() {
            "Ninguna - este usuario no puede crear registros por este chat.".to_string()
        } else {
            acciones.iter()
                .map(|a| format!("- {}: campos {{{}}}", a.entidad, a.campos.join(", ")))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let lista_herramientas = if herramientas.is_empty() {
            "Ninguna - este usuario no puede consultar datos del negocio por este chat.".to_string()
        } else {
            herramientas.iter()
                .map(|h| format!("- {}: parámetros {}", h.nombre, h.parametros))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let prompt = format!(
            "Eres el asistente de Colmado POS, un sistema de punto de venta dominicano. Respondes \
             en español dominicano, breve y amistoso.\n\n\
             REGLA MÁS IMPORTANTE: nunca calcules, estimes o inventes una cifra de negocio por tu \
             cuenta - ni combinando, dividiendo o \"derivando\" otros números que veas en la \
             conversación. La ÚNICA fuente de cifras reales son las herramientas de abajo. Si la \
             pregunta pide un dato de negocio (ventas, gastos, inventario, clientes, caja, etc.) y \
             ninguna herramienta de tu lista lo cubre, di honestamente que no tienes esa información \
             disponible con el rol del usuario - NUNCA \"calcules algo aproximado\" para no dejar la \
             pregunta sin respuesta. Inventar un número es peor que decir que no lo sabes.\n\n\
             Puedes consultar información REAL del negocio usando estas herramientas, y SOLO estas:\n{}\n\n\
             Si la pregunta se responde con una de ellas, responde ÚNICAMENTE con una línea: \
             HERRAMIENTA: {{\"nombre\": \"...\", \"parametros\": {{...}}}} (JSON válido, sin texto adicional).\n\n\
             Puedes ayudar a crear estos tipos de registro, y SOLO estos:\n{}\n\n\
             Si el usuario quiere crear uno de estos, responde ÚNICAMENTE con una línea: \
             ACCION: {{\"entidad\": \"...\", \"campos\": {{...}}}} (JSON válido, sin texto adicional).\n\
             Si pide crear o consultar algo que no está en ninguna lista, explica amablemente que no \
             puedes hacerlo desde el chat con su rol actual - nunca lo sustituyas con un cálculo propio.\n\
             Para cualquier otra cosa (preguntas generales, saludos, etc.) responde con: \
             RESPUESTA: <tu respuesta>\n\n\
             Mensaje del usuario: {}",
            lista_herramientas, lista_acciones, mensaje,
        );

        let texto = match self.generar(&prompt).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Ollama no disponible para el chat: {}", e);
                return ChatResponse::Respuesta {
                    texto: "El asistente no está disponible en este momento. Intenta de nuevo en un momento.".to_string(),
                };
            }
        };

        self.interpretar_y_responder(&texto, tenant_id, mensaje, &acciones, &herramientas, rol).await
    }

    /// Busca `marcador` (p.ej. "ACCION:") en cualquier posición del texto (no
    /// solo al inicio) y extrae del primer `{` al último `}` después de él -
    /// modelos pequeños no siempre respetan "responde ÚNICAMENTE con..." al
    /// pie de la letra, a veces anteponen una frase o dejan texto después del
    /// JSON. Más tolerante sin dejar de caer al respaldo si de verdad no hay JSON.
    fn buscar_marcador_json<'a>(texto: &'a str, marcador: &str) -> Option<&'a str> {
        let pos = texto.find(marcador)?;
        let resto = &texto[pos + marcador.len()..];
        match (resto.find('{'), resto.rfind('}')) {
            (Some(ini), Some(fin)) if fin > ini => Some(&resto[ini..=fin]),
            _ => None,
        }
    }

    async fn interpretar_y_responder(
        &self,
        texto: &str,
        tenant_id: &str,
        mensaje: &str,
        acciones: &[&AccionDisponible],
        herramientas: &[&HerramientaDisponible],
        rol: &str,
    ) -> ChatResponse {
        let texto = texto.trim();

        if let Some(json_str) = Self::buscar_marcador_json(texto, "ACCION:") {
            return match serde_json::from_str::<serde_json::Value>(json_str) {
                Ok(val) => {
                    let entidad = val.get("entidad").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if acciones.iter().any(|a| a.entidad == entidad) {
                        if let Some(campos) = val.get("campos") {
                            return ChatResponse::AccionPropuesta { entidad, campos: campos.clone() };
                        }
                    }
                    tracing::warn!("Acción de IA rechazada (entidad='{}' no permitida para rol={})", entidad, rol);
                    ChatResponse::Respuesta { texto: "No puedo hacer eso desde el chat con tu rol actual.".to_string() }
                }
                Err(e) => {
                    tracing::warn!("No se pudo parsear ACCION de la IA: {} - texto: {}", e, json_str);
                    ChatResponse::Respuesta { texto: "No entendí bien esa acción, ¿puedes darme más detalles?".to_string() }
                }
            };
        }

        if let Some(json_str) = Self::buscar_marcador_json(texto, "HERRAMIENTA:") {
            return match serde_json::from_str::<serde_json::Value>(json_str) {
                Ok(val) => {
                    let nombre = val.get("nombre").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let parametros = val.get("parametros").cloned().unwrap_or_else(|| serde_json::json!({}));
                    if !herramientas.iter().any(|h| h.nombre == nombre) {
                        tracing::warn!("Herramienta de IA rechazada (nombre='{}' no permitida para rol={})", nombre, rol);
                        return ChatResponse::Respuesta { texto: "No tengo esa información disponible con tu rol actual.".to_string() };
                    }
                    match self.ejecutar_herramienta(tenant_id, &nombre, &parametros).await {
                        Ok(hechos) => {
                            // Segunda pasada: redacta la respuesta con los datos
                            // reales ya obtenidos. Si esta pasada falla, los
                            // datos crudos ya son una respuesta correcta -
                            // preferible a perderla por un fallo de estilo.
                            let prompt2 = format!(
                                "Pregunta del usuario: {}\nDatos reales obtenidos: {}\nResponde de forma \
                                 breve y natural en español dominicano usando SOLO estos datos, sin \
                                 inventar nada adicional.",
                                mensaje, hechos,
                            );
                            match self.generar(&prompt2).await {
                                Ok(respuesta) => ChatResponse::Respuesta {
                                    texto: respuesta.trim().trim_start_matches("RESPUESTA:").trim().to_string(),
                                },
                                Err(e) => {
                                    tracing::warn!("Ollama no disponible para redactar la respuesta final, devolviendo datos crudos: {}", e);
                                    ChatResponse::Respuesta { texto: hechos }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Fallo ejecutando la herramienta '{}': {}", nombre, e);
                            ChatResponse::Respuesta { texto: "No pude consultar esa información en este momento.".to_string() }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("No se pudo parsear HERRAMIENTA de la IA: {} - texto: {}", e, json_str);
                    ChatResponse::Respuesta { texto: "No entendí bien esa pregunta, ¿puedes reformularla?".to_string() }
                }
            };
        }

        let limpio = texto.strip_prefix("RESPUESTA:").unwrap_or(texto).trim();
        ChatResponse::Respuesta {
            texto: if limpio.is_empty() { "¿En qué puedo ayudarte?".to_string() } else { limpio.to_string() },
        }
    }

    async fn ejecutar_herramienta(&self, tenant_id: &str, nombre: &str, parametros: &serde_json::Value) -> anyhow::Result<String> {
        let periodo = parametros.get("periodo").and_then(|v| v.as_str()).unwrap_or("hoy").to_string();
        match nombre {
            "ventas_periodo" => self.tool_ventas_periodo(tenant_id, &periodo).await,
            "productos_mas_vendidos" => {
                let limite = parametros.get("limite").and_then(|v| v.as_i64()).unwrap_or(5).clamp(1, 20);
                self.tool_productos_mas_vendidos(tenant_id, &periodo, limite).await
            }
            "producto_info" => {
                let q = parametros.get("nombre_o_sku").and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.tool_producto_info(tenant_id, &q).await
            }
            "gastos_periodo" => self.tool_gastos_periodo(tenant_id, &periodo).await,
            "cliente_info" => {
                let q = parametros.get("nombre").and_then(|v| v.as_str()).unwrap_or("").to_string();
                self.tool_cliente_info(tenant_id, &q).await
            }
            "caja_estado" => self.tool_caja_estado(tenant_id).await,
            "inventario_valor" => self.tool_inventario_valor(tenant_id).await,
            _ => anyhow::bail!("Herramienta desconocida: {}", nombre),
        }
    }

    async fn tool_ventas_periodo(&self, tenant_id: &str, periodo: &str) -> anyhow::Result<String> {
        let (desde, hasta) = resolver_periodo(periodo);
        let row: (Option<Decimal>, i64) = sqlx::query_as(
            "SELECT SUM(total), COUNT(*) FROM ventas WHERE tenant_id = $1 AND created_at >= $2 AND created_at < $3",
        )
        .bind(tenant_id).bind(desde).bind(hasta)
        .fetch_one(&self.pool).await?;
        Ok(format!(
            "Periodo consultado: {}. Total vendido: RD${}. Cantidad de ventas: {}.",
            periodo, row.0.unwrap_or_default(), row.1,
        ))
    }

    async fn tool_productos_mas_vendidos(&self, tenant_id: &str, periodo: &str, limite: i64) -> anyhow::Result<String> {
        let (desde, hasta) = resolver_periodo(periodo);
        let rows: Vec<(String, Decimal, Decimal)> = sqlx::query_as(
            r#"SELECT vi.nombre, SUM(vi.cantidad), SUM(vi.subtotal)
               FROM venta_items vi JOIN ventas v ON v.id = vi.venta_id
               WHERE v.tenant_id = $1 AND v.created_at >= $2 AND v.created_at < $3
               GROUP BY vi.nombre ORDER BY SUM(vi.cantidad) DESC LIMIT $4"#,
        )
        .bind(tenant_id).bind(desde).bind(hasta).bind(limite)
        .fetch_all(&self.pool).await?;
        if rows.is_empty() {
            Ok(format!("No hubo ventas registradas en el periodo '{}'.", periodo))
        } else {
            let lista: Vec<String> = rows.iter()
                .map(|(nombre, cantidad, monto)| format!("{} ({} unidades, RD${})", nombre, cantidad, monto))
                .collect();
            Ok(format!("Productos más vendidos ({}): {}.", periodo, lista.join(", ")))
        }
    }

    async fn tool_producto_info(&self, tenant_id: &str, nombre_o_sku: &str) -> anyhow::Result<String> {
        if nombre_o_sku.trim().is_empty() {
            return Ok("No se especificó qué producto buscar.".to_string());
        }
        let row: Option<(String, String, Decimal, Decimal, Decimal)> = sqlx::query_as(
            "SELECT nombre, sku, stock_actual, stock_minimo, precio_venta FROM productos
             WHERE tenant_id = $1 AND activo = true AND (nombre ILIKE $2 OR sku ILIKE $2) LIMIT 1",
        )
        .bind(tenant_id).bind(format!("%{}%", nombre_o_sku))
        .fetch_optional(&self.pool).await?;
        Ok(match row {
            Some((nombre, sku, stock_actual, stock_minimo, precio)) => format!(
                "{} (SKU {}): stock actual {}, stock mínimo {}, precio de venta RD${}.",
                nombre, sku, stock_actual, stock_minimo, precio,
            ),
            None => format!("No se encontró ningún producto activo que coincida con '{}'.", nombre_o_sku),
        })
    }

    async fn tool_gastos_periodo(&self, tenant_id: &str, periodo: &str) -> anyhow::Result<String> {
        let (desde, hasta) = resolver_periodo(periodo);
        let total: Option<Decimal> = sqlx::query_scalar(
            "SELECT SUM(monto) FROM gastos WHERE tenant_id = $1 AND created_at >= $2 AND created_at < $3",
        )
        .bind(tenant_id).bind(desde).bind(hasta)
        .fetch_one(&self.pool).await?;
        let por_categoria: Vec<(String, Decimal)> = sqlx::query_as(
            "SELECT categoria, SUM(monto) FROM gastos WHERE tenant_id = $1 AND created_at >= $2 AND created_at < $3
             GROUP BY categoria ORDER BY SUM(monto) DESC",
        )
        .bind(tenant_id).bind(desde).bind(hasta)
        .fetch_all(&self.pool).await?;
        let desglose = if por_categoria.is_empty() {
            "sin gastos registrados en el periodo".to_string()
        } else {
            por_categoria.iter().map(|(c, m)| format!("{}: RD${}", c, m)).collect::<Vec<_>>().join(", ")
        };
        Ok(format!("Gastos ({}): total RD${}. Por categoría: {}.", periodo, total.unwrap_or_default(), desglose))
    }

    async fn tool_cliente_info(&self, tenant_id: &str, nombre: &str) -> anyhow::Result<String> {
        if nombre.trim().is_empty() {
            return Ok("No se especificó qué cliente buscar.".to_string());
        }
        let row: Option<(String, Decimal, Decimal, Option<String>)> = sqlx::query_as(
            "SELECT nombre, saldo_pendiente, limite_credito, telefono FROM clientes
             WHERE tenant_id = $1 AND activo = true AND nombre ILIKE $2 LIMIT 1",
        )
        .bind(tenant_id).bind(format!("%{}%", nombre))
        .fetch_optional(&self.pool).await?;
        Ok(match row {
            Some((nombre, saldo, limite, telefono)) => format!(
                "{}: saldo fiado pendiente RD${}, límite de crédito RD${}{}.",
                nombre, saldo, limite,
                telefono.map(|t| format!(", teléfono {}", t)).unwrap_or_default(),
            ),
            None => format!("No se encontró ningún cliente activo que coincida con '{}'.", nombre),
        })
    }

    async fn tool_caja_estado(&self, tenant_id: &str) -> anyhow::Result<String> {
        let sesion: Option<(Decimal, DateTime<Utc>)> = sqlx::query_as(
            "SELECT monto_inicial, abierta_at FROM caja_sesiones WHERE tenant_id = $1 AND estado = 'ABIERTA' ORDER BY abierta_at DESC LIMIT 1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool).await?;
        Ok(match sesion {
            Some((inicial, abierta_at)) => {
                let movimientos: (Option<Decimal>, Option<Decimal>) = sqlx::query_as(
                    "SELECT SUM(monto) FILTER (WHERE tipo = 'INGRESO'), SUM(monto) FILTER (WHERE tipo = 'EGRESO')
                     FROM caja_movimientos WHERE tenant_id = $1 AND created_at >= $2",
                )
                .bind(tenant_id).bind(abierta_at)
                .fetch_one(&self.pool).await?;
                let saldo = inicial + movimientos.0.unwrap_or_default() - movimientos.1.unwrap_or_default();
                format!(
                    "La caja está abierta desde {}. Monto inicial RD${}, saldo actual estimado RD${}.",
                    abierta_at.format("%d/%m/%Y %H:%M"), inicial, saldo,
                )
            }
            None => "La caja está cerrada actualmente.".to_string(),
        })
    }

    async fn tool_inventario_valor(&self, tenant_id: &str) -> anyhow::Result<String> {
        let row: (Option<Decimal>, i64) = sqlx::query_as(
            "SELECT SUM(stock_actual * costo), COUNT(*) FILTER (WHERE stock_actual <= stock_minimo)
             FROM productos WHERE tenant_id = $1 AND activo = true AND tipo = 'PRODUCTO'",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool).await?;
        Ok(format!(
            "Valor total del inventario: RD${}. Productos bajo el mínimo de stock: {}.",
            row.0.unwrap_or_default(), row.1,
        ))
    }
}

#[derive(Debug, Clone, Copy)]
struct AccionDisponible {
    entidad: &'static str,
    campos: &'static [&'static str],
    roles: &'static [&'static str],
}

const ACCIONES: &[AccionDisponible] = &[
    AccionDisponible {
        entidad: "producto",
        campos: &["sku", "nombre", "costo", "precio_venta", "stock_actual", "stock_minimo",
                   "itbis_tipo (GRAVADO_18|GRAVADO_16|EXENTO)", "unidad_medida", "descripcion (opcional)"],
        roles: &["ALMACEN"],
    },
    AccionDisponible {
        entidad: "cliente",
        campos: &["nombre", "rnc_cedula (opcional)", "telefono (opcional)", "email (opcional)", "direccion (opcional)"],
        roles: &["CAJERO"],
    },
    AccionDisponible {
        entidad: "proveedor",
        campos: &["nombre", "rnc (opcional)", "telefono (opcional)", "email (opcional)", "direccion (opcional)", "contacto (opcional)"],
        roles: &["ALMACEN"],
    },
    AccionDisponible {
        entidad: "gasto",
        campos: &["concepto", "categoria (ALQUILER|SERVICIOS|TRANSPORTE|OTROS)", "monto"],
        roles: &["CONTADOR"],
    },
];

/// ADMIN ve las 4 (mismo bypass que `role_guard` en main.rs), cualquier otro
/// rol solo ve las que ya podría crear por la UI normal.
fn acciones_para_rol(rol: &str) -> Vec<&'static AccionDisponible> {
    if rol == "ADMIN" {
        ACCIONES.iter().collect()
    } else {
        ACCIONES.iter().filter(|a| a.roles.contains(&rol)).collect()
    }
}

#[derive(Debug, Clone, Copy)]
struct HerramientaDisponible {
    nombre: &'static str,
    parametros: &'static str,
    roles: &'static [&'static str],
}

/// Cada restricción de rol es un espejo directo de `required_roles` en
/// main.rs para el módulo equivalente - una herramienta de solo lectura no
/// debe poder ver por el chat algo que ese rol no podría ver en la UI normal
/// (p.ej. un CAJERO no ve gastos en ningún lado del producto, tampoco aquí).
const HERRAMIENTAS: &[HerramientaDisponible] = &[
    HerramientaDisponible {
        nombre: "ventas_periodo",
        parametros: "periodo (uno de: hoy, ayer, semana, mes, mes_pasado)",
        roles: &["CAJERO"],
    },
    HerramientaDisponible {
        nombre: "productos_mas_vendidos",
        parametros: "periodo (hoy|ayer|semana|mes|mes_pasado), limite (número, opcional, por defecto 5)",
        roles: &["CAJERO"],
    },
    HerramientaDisponible {
        nombre: "producto_info",
        parametros: "nombre_o_sku (texto a buscar)",
        roles: &["CAJERO", "ALMACEN"],
    },
    HerramientaDisponible {
        nombre: "gastos_periodo",
        parametros: "periodo (hoy|ayer|semana|mes|mes_pasado)",
        roles: &["CONTADOR"],
    },
    HerramientaDisponible {
        nombre: "cliente_info",
        parametros: "nombre (texto a buscar)",
        roles: &["CAJERO"],
    },
    HerramientaDisponible {
        nombre: "caja_estado",
        parametros: "(ninguno)",
        roles: &["CAJERO"],
    },
    HerramientaDisponible {
        nombre: "inventario_valor",
        parametros: "(ninguno)",
        roles: &["CAJERO", "ALMACEN"],
    },
];

fn herramientas_para_rol(rol: &str) -> Vec<&'static HerramientaDisponible> {
    if rol == "ADMIN" {
        HERRAMIENTAS.iter().collect()
    } else {
        HERRAMIENTAS.iter().filter(|h| h.roles.contains(&rol)).collect()
    }
}

/// Resuelve un periodo con nombre fijo a un rango de fechas real en el
/// servidor - nunca se le pide al modelo que calcule "el primer día del mes
/// pasado" como fecha ISO, eso es pedirle aritmética de fechas a un modelo de
/// 3B parámetros, una fuente de errores evitable por completo con este enum.
fn resolver_periodo(periodo: &str) -> (DateTime<Utc>, DateTime<Utc>) {
    let ahora = Utc::now();
    let hoy = ahora.date_naive();
    let inicio_del_dia = |d: chrono::NaiveDate| d.and_hms_opt(0, 0, 0).unwrap().and_utc();

    match periodo {
        "ayer" => {
            let ayer = hoy - chrono::Duration::days(1);
            (inicio_del_dia(ayer), inicio_del_dia(hoy))
        }
        "semana" => (inicio_del_dia(hoy - chrono::Duration::days(7)), ahora),
        "mes" => {
            let inicio = hoy.with_day(1).unwrap();
            (inicio_del_dia(inicio), ahora)
        }
        "mes_pasado" => {
            let primero_este_mes = hoy.with_day(1).unwrap();
            let ultimo_mes_pasado = primero_este_mes - chrono::Duration::days(1);
            let primero_mes_pasado = ultimo_mes_pasado.with_day(1).unwrap();
            (inicio_del_dia(primero_mes_pasado), inicio_del_dia(primero_este_mes))
        }
        // "hoy" y cualquier valor no reconocido - por defecto, hoy.
        _ => (inicio_del_dia(hoy), ahora),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum ChatResponse {
    Respuesta { texto: String },
    AccionPropuesta { entidad: String, campos: serde_json::Value },
}
