use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/fiscal_core".to_string());
    let pool = PgPool::connect(&url).await?;

    println!("Running migrations...");

    // Use raw_sql for multiple statements (sqlx 0.8+)
    // Or split into separate queries if raw_sql not available
    sqlx::raw_sql(
        r#"
        CREATE EXTENSION IF NOT EXISTS "pgcrypto";

        -- Orphaned leftovers from the removed event-sourcing/TigerBeetle design.
        -- Nothing in services/core reads or writes these; drop them for real.
        DROP TABLE IF EXISTS snapshots;
        DROP TABLE IF EXISTS read_employee_balances;
        DROP TABLE IF EXISTS read_sales;

        CREATE TABLE IF NOT EXISTS events (
            event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            aggregate_type TEXT NOT NULL,
            aggregate_id UUID NOT NULL,
            version BIGINT NOT NULL,
            event_type TEXT NOT NULL,
            payload JSONB NOT NULL,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            tenant_id TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            prev_hash TEXT,
            hash TEXT NOT NULL,
            UNIQUE(aggregate_id, version)
        );

        CREATE INDEX IF NOT EXISTS idx_events_aggregate ON events (aggregate_id, version);
        CREATE INDEX IF NOT EXISTS idx_events_tenant_type ON events (tenant_id, aggregate_type);

        -- MODULO 1: Auth y Multi-tenancy
        CREATE TABLE IF NOT EXISTS tenants (
            rnc TEXT PRIMARY KEY,
            razon_social TEXT NOT NULL,
            nombre_comercial TEXT,
            direccion TEXT NOT NULL,
            telefono TEXT,
            correo TEXT,
            logo_url TEXT,
            ambiente_dgii TEXT NOT NULL DEFAULT 'TesteCF',
            factura_electronica_activa BOOLEAN NOT NULL DEFAULT true,
            license_status TEXT NOT NULL DEFAULT 'trial', -- trial | active | expired
            trial_started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            trial_days INT NOT NULL DEFAULT 90,
            license_activated_at TIMESTAMPTZ,
            last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            license_sig TEXT NOT NULL DEFAULT '',
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        -- Idempotent for databases that already ran the migration before this
        -- column existed (CREATE TABLE IF NOT EXISTS above is a no-op for them).
        ALTER TABLE tenants ADD COLUMN IF NOT EXISTS factura_electronica_activa BOOLEAN NOT NULL DEFAULT true;
        ALTER TABLE tenants ADD COLUMN IF NOT EXISTS license_status TEXT NOT NULL DEFAULT 'trial';
        ALTER TABLE tenants ADD COLUMN IF NOT EXISTS trial_started_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
        ALTER TABLE tenants ADD COLUMN IF NOT EXISTS trial_days INT NOT NULL DEFAULT 90;
        ALTER TABLE tenants ADD COLUMN IF NOT EXISTS license_activated_at TIMESTAMPTZ;
        ALTER TABLE tenants ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
        ALTER TABLE tenants ADD COLUMN IF NOT EXISTS license_sig TEXT NOT NULL DEFAULT '';

        CREATE TABLE IF NOT EXISTS usuarios (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            nombre TEXT NOT NULL,
            email TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            rol TEXT NOT NULL DEFAULT 'CAJERO', -- ADMIN, CAJERO, ALMACEN, CONTADOR
            descuento_maximo_sin_aprobacion NUMERIC NOT NULL DEFAULT 0,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(tenant_id, email)
        );

        ALTER TABLE usuarios ADD COLUMN IF NOT EXISTS descuento_maximo_sin_aprobacion NUMERIC NOT NULL DEFAULT 0;

        CREATE INDEX IF NOT EXISTS idx_usuarios_tenant ON usuarios(tenant_id);
        CREATE INDEX IF NOT EXISTS idx_usuarios_email ON usuarios(email);

        -- Tabla secuencias DGII e-NCF por tenant
        CREATE TABLE IF NOT EXISTS secuencias_ncf (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            tipo_ecf INT NOT NULL, -- 31,32,33,34,41,43, etc
            prefijo TEXT NOT NULL, -- E31, E32...
            desde BIGINT NOT NULL,
            hasta BIGINT NOT NULL,
            proximo BIGINT NOT NULL,
            fecha_vencimiento DATE NOT NULL,
            estado TEXT NOT NULL DEFAULT 'ACTIVA', -- ACTIVA, AGOTADA, VENCIDA
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_secuencias_tenant_tipo ON secuencias_ncf(tenant_id, tipo_ecf);

        -- MODULO 2: Categorias y Productos (plain Postgres, no event sourcing / ledger)
        CREATE TABLE IF NOT EXISTS categorias (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            nombre TEXT NOT NULL,
            color TEXT,
            icono TEXT,
            orden INT NOT NULL DEFAULT 0,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS productos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            categoria_id UUID REFERENCES categorias(id) ON DELETE SET NULL,
            sku TEXT NOT NULL,
            codigo_barras TEXT,
            nombre TEXT NOT NULL,
            descripcion TEXT,
            unidad_medida TEXT NOT NULL DEFAULT '43',
            itbis_tipo TEXT NOT NULL DEFAULT 'GRAVADO_18', -- GRAVADO_18 | GRAVADO_16 | EXENTO
            costo DECIMAL(12,2) NOT NULL DEFAULT 0,
            precio_venta DECIMAL(12,2) NOT NULL DEFAULT 0,
            stock_actual DECIMAL(12,2) NOT NULL DEFAULT 0,
            stock_minimo DECIMAL(12,2) NOT NULL DEFAULT 0,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(tenant_id, sku)
        );

        CREATE INDEX IF NOT EXISTS idx_productos_tenant ON productos(tenant_id);
        CREATE INDEX IF NOT EXISTS idx_categorias_tenant ON categorias(tenant_id);

        -- Miniatura del producto (ver image_service.rs) - se guarda solo la
        -- ruta servida estáticamente, nunca el binario en la fila.
        ALTER TABLE productos ADD COLUMN IF NOT EXISTS imagen_url TEXT;

        -- MODULO 3: Inventario (kardex) - plain movement log, adjusts productos.stock_actual
        CREATE TABLE IF NOT EXISTS movimientos_inventario (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            producto_id UUID NOT NULL REFERENCES productos(id) ON DELETE CASCADE,
            tipo TEXT NOT NULL, -- ENTRADA | SALIDA | AJUSTE
            cantidad DECIMAL(12,2) NOT NULL, -- signed delta actually applied to stock_actual (+ increases, - decreases)
            costo_unitario DECIMAL(12,2),
            motivo TEXT,
            referencia_tipo TEXT, -- COMPRA | VENTA | AJUSTE_MANUAL
            referencia_id UUID,
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_movimientos_tenant ON movimientos_inventario(tenant_id);
        CREATE INDEX IF NOT EXISTS idx_movimientos_producto ON movimientos_inventario(producto_id, created_at DESC);

        -- MODULO 4: Clientes y Proveedores
        CREATE TABLE IF NOT EXISTS clientes (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            nombre TEXT NOT NULL,
            rnc_cedula TEXT,
            telefono TEXT,
            email TEXT,
            direccion TEXT,
            saldo_pendiente DECIMAL(12,2) NOT NULL DEFAULT 0,
            limite_credito DECIMAL(12,2) NOT NULL DEFAULT 0,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        ALTER TABLE clientes ADD COLUMN IF NOT EXISTS saldo_pendiente DECIMAL(12,2) NOT NULL DEFAULT 0;
        ALTER TABLE clientes ADD COLUMN IF NOT EXISTS limite_credito DECIMAL(12,2) NOT NULL DEFAULT 0;

        CREATE TABLE IF NOT EXISTS cliente_abonos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            cliente_id UUID NOT NULL REFERENCES clientes(id) ON DELETE CASCADE,
            monto DECIMAL(12,2) NOT NULL,
            metodo_pago TEXT NOT NULL DEFAULT 'EFECTIVO',
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_cliente_abonos_cliente ON cliente_abonos(cliente_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS proveedores (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            nombre TEXT NOT NULL,
            rnc TEXT,
            telefono TEXT,
            email TEXT,
            direccion TEXT,
            contacto TEXT,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_clientes_tenant ON clientes(tenant_id);
        CREATE INDEX IF NOT EXISTS idx_proveedores_tenant ON proveedores(tenant_id);

        -- Proveedor principal del producto (proveedores ya existe arriba).
        ALTER TABLE productos ADD COLUMN IF NOT EXISTS proveedor_id UUID REFERENCES proveedores(id) ON DELETE SET NULL;
        CREATE INDEX IF NOT EXISTS idx_productos_proveedor ON productos(proveedor_id);

        -- MODULO 5: Ventas / Punto de Venta
        CREATE TABLE IF NOT EXISTS ventas (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            cliente_id UUID REFERENCES clientes(id) ON DELETE SET NULL,
            usuario_id UUID REFERENCES usuarios(id),
            subtotal DECIMAL(12,2) NOT NULL,
            itbis_total DECIMAL(12,2) NOT NULL,
            total DECIMAL(12,2) NOT NULL,
            metodo_pago TEXT NOT NULL DEFAULT 'EFECTIVO', -- EFECTIVO | TARJETA | CREDITO | TRANSFERENCIA | FIADO
            estado TEXT NOT NULL DEFAULT 'COMPLETADA', -- COMPLETADA | ANULADA
            e_ncf TEXT,
            tipo_ecf INT,
            estado_dgii TEXT, -- NULL hasta que se emita: PENDIENTE | ACEPTADO | RECHAZADO
            track_id TEXT,
            codigo_seguridad TEXT,
            qr_url TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS venta_items (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            venta_id UUID NOT NULL REFERENCES ventas(id) ON DELETE CASCADE,
            producto_id UUID NOT NULL REFERENCES productos(id),
            sku TEXT NOT NULL,
            nombre TEXT NOT NULL,
            cantidad DECIMAL(12,2) NOT NULL,
            precio_unitario DECIMAL(12,2) NOT NULL,
            descuento DECIMAL(12,2) NOT NULL DEFAULT 0,
            itbis_tipo TEXT NOT NULL,
            itbis_monto DECIMAL(12,2) NOT NULL,
            subtotal DECIMAL(12,2) NOT NULL
        );

        ALTER TABLE venta_items ADD COLUMN IF NOT EXISTS descuento DECIMAL(12,2) NOT NULL DEFAULT 0;

        -- MODULO 9 (tabla creada aqui porque Ventas ya necesita registrar ingresos de caja)
        CREATE TABLE IF NOT EXISTS caja_movimientos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            tipo TEXT NOT NULL, -- INGRESO | EGRESO
            concepto TEXT NOT NULL,
            monto DECIMAL(12,2) NOT NULL,
            metodo_pago TEXT,
            referencia_tipo TEXT, -- VENTA | COMPRA | AJUSTE_MANUAL
            referencia_id UUID,
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_ventas_tenant ON ventas(tenant_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_venta_items_venta ON venta_items(venta_id);
        CREATE INDEX IF NOT EXISTS idx_caja_movimientos_tenant ON caja_movimientos(tenant_id, created_at DESC);

        -- MODULO 6: Compras y Gastos
        CREATE TABLE IF NOT EXISTS compras (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            proveedor_id UUID REFERENCES proveedores(id) ON DELETE SET NULL,
            usuario_id UUID REFERENCES usuarios(id),
            ncf_proveedor TEXT,
            subtotal DECIMAL(12,2) NOT NULL,
            itbis_total DECIMAL(12,2) NOT NULL,
            total DECIMAL(12,2) NOT NULL,
            metodo_pago TEXT NOT NULL DEFAULT 'EFECTIVO',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS compra_items (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            compra_id UUID NOT NULL REFERENCES compras(id) ON DELETE CASCADE,
            producto_id UUID NOT NULL REFERENCES productos(id),
            sku TEXT NOT NULL,
            nombre TEXT NOT NULL,
            cantidad DECIMAL(12,2) NOT NULL,
            costo_unitario DECIMAL(12,2) NOT NULL,
            itbis_monto DECIMAL(12,2) NOT NULL,
            subtotal DECIMAL(12,2) NOT NULL
        );

        CREATE TABLE IF NOT EXISTS gastos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            concepto TEXT NOT NULL,
            categoria TEXT NOT NULL DEFAULT 'OTROS', -- ALQUILER | SERVICIOS | TRANSPORTE | OTROS
            monto DECIMAL(12,2) NOT NULL,
            proveedor_id UUID REFERENCES proveedores(id) ON DELETE SET NULL,
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_compras_tenant ON compras(tenant_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_compra_items_compra ON compra_items(compra_id);
        CREATE INDEX IF NOT EXISTS idx_gastos_tenant ON gastos(tenant_id, created_at DESC);

        -- MODULO 9: Caja y Bancos
        CREATE TABLE IF NOT EXISTS caja_sesiones (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            usuario_id UUID REFERENCES usuarios(id),
            monto_inicial DECIMAL(12,2) NOT NULL,
            monto_final DECIMAL(12,2),
            monto_esperado DECIMAL(12,2),
            diferencia DECIMAL(12,2),
            estado TEXT NOT NULL DEFAULT 'ABIERTA', -- ABIERTA | CERRADA
            abierta_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            cerrada_at TIMESTAMPTZ
        );

        CREATE TABLE IF NOT EXISTS bancos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            nombre_banco TEXT NOT NULL,
            numero_cuenta TEXT,
            tipo_cuenta TEXT NOT NULL DEFAULT 'CORRIENTE', -- CORRIENTE | AHORROS
            saldo DECIMAL(12,2) NOT NULL DEFAULT 0,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS banco_movimientos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            banco_id UUID NOT NULL REFERENCES bancos(id) ON DELETE CASCADE,
            tipo TEXT NOT NULL, -- DEPOSITO | RETIRO
            concepto TEXT,
            monto DECIMAL(12,2) NOT NULL,
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_caja_sesiones_tenant ON caja_sesiones(tenant_id, abierta_at DESC);
        CREATE INDEX IF NOT EXISTS idx_bancos_tenant ON bancos(tenant_id);
        CREATE INDEX IF NOT EXISTS idx_banco_movimientos_banco ON banco_movimientos(banco_id, created_at DESC);

        -- MODULO 8: Nomina y Adelantos (reconstruido plano; la version anterior
        -- vivia en el aggregates/employee.rs de event-sourcing ya eliminado)
        CREATE TABLE IF NOT EXISTS empleados (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            nombre TEXT NOT NULL,
            cedula TEXT,
            puesto TEXT,
            salario_mensual DECIMAL(12,2) NOT NULL,
            fecha_ingreso DATE,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS nomina_adelantos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            empleado_id UUID NOT NULL REFERENCES empleados(id) ON DELETE CASCADE,
            monto DECIMAL(12,2) NOT NULL,
            motivo TEXT,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE', -- PENDIENTE | APROBADO | RECHAZADO | DESCONTADO
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS nomina_periodos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            periodo TEXT NOT NULL,
            fecha_pago DATE,
            total_bruto DECIMAL(12,2) NOT NULL DEFAULT 0,
            total_neto DECIMAL(12,2) NOT NULL DEFAULT 0,
            estado TEXT NOT NULL DEFAULT 'PAGADO',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS nomina_detalles (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            periodo_id UUID NOT NULL REFERENCES nomina_periodos(id) ON DELETE CASCADE,
            empleado_id UUID NOT NULL REFERENCES empleados(id),
            empleado_nombre TEXT NOT NULL,
            salario_bruto DECIMAL(12,2) NOT NULL,
            tss DECIMAL(12,2) NOT NULL DEFAULT 0,
            isr DECIMAL(12,2) NOT NULL DEFAULT 0,
            adelantos_descuento DECIMAL(12,2) NOT NULL DEFAULT 0,
            neto DECIMAL(12,2) NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_empleados_tenant ON empleados(tenant_id);
        CREATE INDEX IF NOT EXISTS idx_adelantos_empleado ON nomina_adelantos(empleado_id, estado);
        CREATE INDEX IF NOT EXISTS idx_nomina_periodos_tenant ON nomina_periodos(tenant_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_nomina_detalles_periodo ON nomina_detalles(periodo_id);

        -- MODULO 7: Contabilidad (Libro Mayor) - doble entrada plana, sin TigerBeetle.
        -- Las filas se crean a mano (asiento manual) o via /v1/contabilidad/sincronizar,
        -- que genera asientos simples para Ventas/Compras/Nomina que aun no los tienen.
        CREATE TABLE IF NOT EXISTS asientos_contables (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            fecha DATE NOT NULL DEFAULT CURRENT_DATE,
            cuenta TEXT NOT NULL,
            descripcion TEXT NOT NULL,
            debe DECIMAL(12,2) NOT NULL DEFAULT 0,
            haber DECIMAL(12,2) NOT NULL DEFAULT 0,
            referencia_tipo TEXT, -- VENTA | COMPRA | NOMINA | GASTO | MANUAL
            referencia_id UUID,
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_asientos_tenant ON asientos_contables(tenant_id, fecha DESC);
        CREATE INDEX IF NOT EXISTS idx_asientos_cuenta ON asientos_contables(tenant_id, cuenta);
        CREATE INDEX IF NOT EXISTS idx_asientos_referencia ON asientos_contables(referencia_tipo, referencia_id);

        -- MODULO 11: Configuracion DGII y Empresa
        CREATE TABLE IF NOT EXISTS certificados_dgii (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            nombre_archivo TEXT NOT NULL,
            cert_encrypted BYTEA NOT NULL,
            cert_nonce BYTEA NOT NULL,
            password_encrypted BYTEA NOT NULL,
            password_nonce BYTEA NOT NULL,
            activo BOOLEAN NOT NULL DEFAULT true,
            uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_certificados_tenant_activo ON certificados_dgii(tenant_id, activo);

        CREATE TABLE IF NOT EXISTS impresora_config (
            tenant_id TEXT PRIMARY KEY REFERENCES tenants(rnc) ON DELETE CASCADE,
            ip TEXT,
            puerto INT NOT NULL DEFAULT 9100,
            ancho_mm INT NOT NULL DEFAULT 80,
            copias INT NOT NULL DEFAULT 1,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        -- Padron de contribuyentes DGII (RNC/Cedula) - dato publico nacional,
        -- NO tenant-scoped a proposito: es el mismo registro para todos los
        -- tenants. Se llena vía `cargo run --bin import_rnc` (descarga el
        -- archivo oficial DGII_RNC.zip de dgii.gov.do).
        CREATE TABLE IF NOT EXISTS rnc_padron (
            rnc TEXT PRIMARY KEY,
            nombre TEXT NOT NULL,
            nombre_comercial TEXT,
            actividad_economica TEXT,
            fecha_inicio DATE,
            estado TEXT,
            tipo_contribuyente TEXT,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        -- Migracion NCF -> e-CF real: registro de cada e-CF firmado (Ventas o
        -- Notas de Credito), con el XML firmado completo para la retencion de
        -- 10 anos exigida por DGII. Antes el XML se firmaba y se descartaba.
        CREATE TABLE IF NOT EXISTS ecf_documentos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            referencia_tipo TEXT NOT NULL, -- VENTA | NOTA_CREDITO
            referencia_id UUID NOT NULL,
            tipo_ecf INT NOT NULL,
            e_ncf TEXT NOT NULL,
            xml_firmado TEXT NOT NULL,
            estado_dgii TEXT NOT NULL, -- FIRMADO_PENDIENTE_ENVIO | ACEPTADO | RECHAZADO | CONTINGENCIA_PENDIENTE
            track_id TEXT,
            codigo_seguridad TEXT,
            qr_url TEXT,
            mensaje_dgii TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_ecf_documentos_tenant ON ecf_documentos(tenant_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_ecf_documentos_referencia ON ecf_documentos(referencia_tipo, referencia_id);
        CREATE INDEX IF NOT EXISTS idx_ecf_documentos_pendientes ON ecf_documentos(tenant_id, estado_dgii);

        -- Notas de Credito (e-CF Tipo 34): documento fiscal propio que
        -- referencia la venta original, en vez de editarla/anularla en sitio.
        CREATE TABLE IF NOT EXISTS notas_credito (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            venta_id UUID NOT NULL REFERENCES ventas(id),
            usuario_id UUID REFERENCES usuarios(id),
            motivo TEXT NOT NULL,
            subtotal DECIMAL(12,2) NOT NULL,
            itbis_total DECIMAL(12,2) NOT NULL,
            total DECIMAL(12,2) NOT NULL,
            e_ncf TEXT,
            estado_dgii TEXT,
            codigo_seguridad TEXT,
            qr_url TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_notas_credito_venta ON notas_credito(venta_id);

        -- Bitacora de auditoria: quien hizo que, para las acciones de mayor
        -- impacto (ventas/descuentos, inventario, usuarios, nomina, licencia).
        -- No se registra cada GET/lectura, solo mutaciones de valor.
        CREATE TABLE IF NOT EXISTS auditoria (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            usuario_id UUID REFERENCES usuarios(id),
            accion TEXT NOT NULL,
            entidad TEXT NOT NULL,
            entidad_id UUID,
            detalle JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_auditoria_tenant ON auditoria(tenant_id, created_at DESC);

        -- Reset de contraseña por correo (self-serve). El token real solo
        -- existe en el enlace enviado por correo - aqui se guarda su hash
        -- (mismo principio que password_hash en usuarios), de un solo uso.
        CREATE TABLE IF NOT EXISTS password_reset_tokens (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            usuario_id UUID NOT NULL REFERENCES usuarios(id) ON DELETE CASCADE,
            token_hash TEXT NOT NULL,
            expires_at TIMESTAMPTZ NOT NULL,
            used_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_hash ON password_reset_tokens(token_hash);

        -- Cotizaciones: propuesta sin compromiso, no toca stock ni caja hasta
        -- que se convierte en una Venta real (ver ventas_service::create_venta,
        -- reutilizado tal cual para la conversión).
        CREATE TABLE IF NOT EXISTS cotizaciones (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            cliente_id UUID REFERENCES clientes(id) ON DELETE SET NULL,
            usuario_id UUID REFERENCES usuarios(id),
            subtotal DECIMAL(12,2) NOT NULL,
            itbis_total DECIMAL(12,2) NOT NULL,
            total DECIMAL(12,2) NOT NULL,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE', -- PENDIENTE | ACEPTADA | RECHAZADA | CONVERTIDA | VENCIDA
            fecha_vencimiento DATE,
            venta_id UUID REFERENCES ventas(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS cotizacion_items (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            cotizacion_id UUID NOT NULL REFERENCES cotizaciones(id) ON DELETE CASCADE,
            producto_id UUID NOT NULL REFERENCES productos(id),
            sku TEXT NOT NULL,
            nombre TEXT NOT NULL,
            cantidad DECIMAL(12,2) NOT NULL,
            precio_unitario DECIMAL(12,2) NOT NULL,
            descuento DECIMAL(12,2) NOT NULL DEFAULT 0,
            itbis_tipo TEXT NOT NULL,
            itbis_monto DECIMAL(12,2) NOT NULL,
            subtotal DECIMAL(12,2) NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_cotizaciones_tenant ON cotizaciones(tenant_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_cotizacion_items_cotizacion ON cotizacion_items(cotizacion_id);

        -- Conduces (guías de despacho): a diferencia del diseño anterior, un
        -- conduce ya NO es un documento con precio propio que luego se
        -- "convierte" en Venta. La Venta (factura) se crea PRIMERO, por la
        -- cantidad total acordada; el conduce solo registra que una parte de
        -- esa cantidad salió físicamente del negocio - sin precio, sin ITBIS,
        -- solo cantidad. Ver ventas.entrega_diferida / venta_items.cantidad_entregada.
        DROP TABLE IF EXISTS conduce_items;
        DROP TABLE IF EXISTS conduces;

        CREATE TABLE conduces (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            venta_id UUID NOT NULL REFERENCES ventas(id),
            usuario_id UUID REFERENCES usuarios(id),
            direccion_entrega TEXT,
            -- Campos del formato de conduce impreso (guía de despacho física):
            -- ninguno tiene precio, son puramente de logística/entrega.
            orden_compra TEXT,
            vehiculo_placa TEXT,
            conductor TEXT,
            notas TEXT,
            entregado_por TEXT,
            recibido_por TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE conduce_items (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            conduce_id UUID NOT NULL REFERENCES conduces(id) ON DELETE CASCADE,
            venta_item_id UUID NOT NULL REFERENCES venta_items(id),
            producto_id UUID NOT NULL REFERENCES productos(id),
            sku TEXT NOT NULL,
            nombre TEXT NOT NULL,
            cantidad DECIMAL(12,2) NOT NULL,
            unidad TEXT,
            observaciones TEXT
        );

        CREATE INDEX idx_conduces_tenant ON conduces(tenant_id, created_at DESC);
        CREATE INDEX idx_conduces_venta ON conduces(venta_id);
        CREATE INDEX idx_conduce_items_conduce ON conduce_items(conduce_id);

        -- entrega_diferida: la mercancía NO sale toda al facturar - sale en
        -- lotes vía conduces. cantidad_entregada es cuánto de esa línea ha
        -- salido realmente del inventario hasta ahora (para venta normal,
        -- queda igual a `cantidad` desde el momento de la venta - ver
        -- ventas_service::create_venta).
        ALTER TABLE ventas ADD COLUMN IF NOT EXISTS entrega_diferida BOOLEAN NOT NULL DEFAULT false;
        ALTER TABLE venta_items ADD COLUMN IF NOT EXISTS cantidad_entregada DECIMAL(12,2) NOT NULL DEFAULT 0;
        "#
    )
    .execute(&pool)
    .await?;

    println!("Migrations OK - EventStore + read models created");
    Ok(())
}
