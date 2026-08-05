//! Limitador de intentos en memoria - suficiente para un proceso único por
//! negocio (no hay múltiples instancias detrás de un balanceador que
//! necesiten compartir este estado). Usado para bloquear fuerza bruta en
//! login y para no dejar que alguien spamee el endpoint de "olvidé mi
//! contraseña" (que dispara un envío de correo real).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Entry {
    fallos: u32,
    primer_fallo: Instant,
}

pub struct RateLimiter {
    intentos: Mutex<HashMap<String, Entry>>,
    max_intentos: u32,
    ventana: Duration,
}

impl RateLimiter {
    pub fn new(max_intentos: u32, ventana: Duration) -> Self {
        Self { intentos: Mutex::new(HashMap::new()), max_intentos, ventana }
    }

    /// `true` si `key` ya superó el límite dentro de la ventana actual - el
    /// caller no debe ni intentar la operación (ni gastar un intento de
    /// verificación de contraseña) cuando esto da `true`.
    pub fn bloqueado(&self, key: &str) -> bool {
        let mut mapa = self.intentos.lock().unwrap();
        if let Some(entry) = mapa.get(key) {
            if entry.primer_fallo.elapsed() > self.ventana {
                mapa.remove(key);
                return false;
            }
            return entry.fallos >= self.max_intentos;
        }
        false
    }

    /// Registra un fallo (login incorrecto, etc). Llamar solo después de
    /// confirmar que `bloqueado` era `false`.
    pub fn registrar_fallo(&self, key: &str) {
        let mut mapa = self.intentos.lock().unwrap();
        let entry = mapa.entry(key.to_string()).or_insert_with(|| Entry { fallos: 0, primer_fallo: Instant::now() });
        if entry.primer_fallo.elapsed() > self.ventana {
            entry.fallos = 0;
            entry.primer_fallo = Instant::now();
        }
        entry.fallos += 1;
    }

    /// Limpia el contador - llamar en cada éxito.
    pub fn limpiar(&self, key: &str) {
        self.intentos.lock().unwrap().remove(key);
    }
}
