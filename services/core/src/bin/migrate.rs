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

        -- Orphaned leftovers from the removed event-sourcing design.
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
        ALTER TABLE tenants ADD COLUMN IF NOT EXISTS tipo_negocio TEXT NOT NULL DEFAULT 'COLMADO';
        DO $$ BEGIN
            ALTER TABLE tenants ADD CONSTRAINT chk_tenants_tipo_negocio
                CHECK (tipo_negocio IN ('COLMADO', 'SERVICIOS'));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;

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

        -- Catálogo de Servicio: item sin stock ni precio fijo (el precio se
        -- captura por línea al cotizar/facturar, ver ventas_service y
        -- cotizacion_service). PRODUCTO sigue siendo el default - byte
        -- idéntico para todo el catálogo existente.
        ALTER TABLE productos ADD COLUMN IF NOT EXISTS tipo TEXT NOT NULL DEFAULT 'PRODUCTO';
        DO $$ BEGIN
            ALTER TABLE productos ADD CONSTRAINT chk_productos_tipo
                CHECK (tipo IN ('PRODUCTO', 'SERVICIO'));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;
        ALTER TABLE productos ALTER COLUMN precio_venta DROP NOT NULL;
        DO $$ BEGIN
            ALTER TABLE productos ADD CONSTRAINT chk_productos_precio_venta_por_tipo
                CHECK ((tipo = 'PRODUCTO' AND precio_venta IS NOT NULL) OR (tipo = 'SERVICIO' AND precio_venta IS NULL));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;

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
            metodo_pago TEXT NOT NULL DEFAULT 'EFECTIVO', -- EFECTIVO | TARJETA | TRANSFERENCIA | FIADO
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

        -- MODULO 7: Contabilidad (Libro Mayor) - doble entrada plana en Postgres.
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

        -- Conduces (guías de despacho / "Órdenes de Servicio" para tenants
        -- SERVICIOS): a diferencia del diseño anterior, un conduce ya NO es
        -- un documento con precio propio que luego se "convierte" en Venta.
        -- La Venta (factura) se crea PRIMERO, por la cantidad total
        -- acordada; el conduce solo registra que una parte de esa cantidad
        -- salió físicamente del negocio - sin precio, sin ITBIS, solo
        -- cantidad. Ver ventas.entrega_diferida / venta_items.cantidad_entregada.
        -- venta_id/venta_item_id son nullable: un conduce también puede
        -- crearse standalone (sin Venta previa), en cuyo caso cliente_id
        -- identifica al cliente y producto_id/sku/nombre/cantidad en
        -- conduce_items se llenan directo en vez de copiarse del venta_item.
        -- NOTA: hasta esta migración esta tabla se recreaba con DROP+CREATE
        -- en cada corrida de `migrate` (perdiendo datos reales cada vez) -
        -- se cambia aquí al mismo patrón idempotente CREATE IF NOT EXISTS +
        -- ALTER ADD COLUMN IF NOT EXISTS usado en el resto del archivo.
        CREATE TABLE IF NOT EXISTS conduces (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            venta_id UUID REFERENCES ventas(id),
            cliente_id UUID REFERENCES clientes(id) ON DELETE SET NULL,
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
        ALTER TABLE conduces ALTER COLUMN venta_id DROP NOT NULL;
        ALTER TABLE conduces ADD COLUMN IF NOT EXISTS cliente_id UUID REFERENCES clientes(id) ON DELETE SET NULL;

        CREATE TABLE IF NOT EXISTS conduce_items (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            conduce_id UUID NOT NULL REFERENCES conduces(id) ON DELETE CASCADE,
            venta_item_id UUID REFERENCES venta_items(id),
            producto_id UUID NOT NULL REFERENCES productos(id),
            sku TEXT NOT NULL,
            nombre TEXT NOT NULL,
            cantidad DECIMAL(12,2) NOT NULL,
            unidad TEXT,
            observaciones TEXT
        );
        ALTER TABLE conduce_items ALTER COLUMN venta_item_id DROP NOT NULL;

        CREATE INDEX IF NOT EXISTS idx_conduces_tenant ON conduces(tenant_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conduces_venta ON conduces(venta_id);
        CREATE INDEX IF NOT EXISTS idx_conduce_items_conduce ON conduce_items(conduce_id);

        -- entrega_diferida: la mercancía NO sale toda al facturar - sale en
        -- lotes vía conduces. cantidad_entregada es cuánto de esa línea ha
        -- salido realmente del inventario hasta ahora (para venta normal,
        -- queda igual a `cantidad` desde el momento de la venta - ver
        -- ventas_service::create_venta).
        ALTER TABLE ventas ADD COLUMN IF NOT EXISTS entrega_diferida BOOLEAN NOT NULL DEFAULT false;
        ALTER TABLE venta_items ADD COLUMN IF NOT EXISTS cantidad_entregada DECIMAL(12,2) NOT NULL DEFAULT 0;

        -- CREDITO era un valor documentado en metodo_pago pero nunca tuvo
        -- lógica propia en la app (solo FIADO se chequea en
        -- ventas_service::create_venta) - el frontend ya no lo ofrece.
        -- Compras tampoco distingue metodo_pago (siempre paga de contado,
        -- ver compras_service::create_compra), así que se restringe igual.
        DO $$ BEGIN
            ALTER TABLE ventas ADD CONSTRAINT chk_ventas_metodo_pago
                CHECK (metodo_pago IN ('EFECTIVO', 'TARJETA', 'TRANSFERENCIA', 'FIADO'));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;
        -- FIADO en compras: a diferencia del CREDITO nunca-implementado de
        -- arriba, esto sí tiene lógica propia (ver compras_service.rs y la
        -- rama FIADO de contabilidad_service::sincronizar) - una compra fiada
        -- no mueve caja, se acredita 2110 Cuentas por Pagar en su lugar.
        ALTER TABLE compras DROP CONSTRAINT IF EXISTS chk_compras_metodo_pago;
        ALTER TABLE compras ADD CONSTRAINT chk_compras_metodo_pago
            CHECK (metodo_pago IN ('EFECTIVO', 'TARJETA', 'TRANSFERENCIA', 'FIADO'));
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS fecha_vencimiento DATE;

        -- MODULO 7b: Plan de cuentas, cabecera de asientos y periodos
        -- contables. Ver docs/12-LIBRO-DIARIO-LIBRO-MAYOR-PLAN.md.

        CREATE TABLE IF NOT EXISTS periodos_contables (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            anio INT NOT NULL,
            mes INT NOT NULL,
            estado TEXT NOT NULL DEFAULT 'ABIERTO', -- ABIERTO | CERRADO
            cerrado_at TIMESTAMPTZ,
            cerrado_por UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(tenant_id, anio, mes)
        );
        CREATE INDEX IF NOT EXISTS idx_periodos_tenant ON periodos_contables(tenant_id, anio DESC, mes DESC);

        CREATE TABLE IF NOT EXISTS asientos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            fecha DATE NOT NULL DEFAULT CURRENT_DATE,
            descripcion TEXT NOT NULL,
            origen TEXT NOT NULL, -- AUTOMATICO | MANUAL | REVERSION
            referencia_tipo TEXT, -- VENTA | COMPRA | NOMINA | GASTO | ADELANTO |
                                  -- ABONO_CLIENTE | NOTA_CREDITO | AJUSTE_INVENTARIO |
                                  -- BANCO | MANUAL
            referencia_id UUID,
            reversa_de UUID REFERENCES asientos(id),
            periodo_id UUID REFERENCES periodos_contables(id),
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(tenant_id, referencia_tipo, referencia_id)
        );
        CREATE INDEX IF NOT EXISTS idx_asientos_tenant_fecha ON asientos(tenant_id, fecha DESC, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_asientos_reversa ON asientos(reversa_de);

        CREATE TABLE IF NOT EXISTS cuentas_contables (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            codigo TEXT NOT NULL,
            nombre TEXT NOT NULL,
            tipo TEXT NOT NULL, -- ACTIVO | PASIVO | PATRIMONIO | INGRESO | GASTO
            naturaleza TEXT NOT NULL, -- DEUDORA | ACREEDORA
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(tenant_id, codigo)
        );
        CREATE INDEX IF NOT EXISTS idx_cuentas_contables_tenant ON cuentas_contables(tenant_id, activo);

        -- Semilla del plan de cuentas para tenants existentes (los nuevos se
        -- siembran en auth_service::register_tenant). Idempotente vía el
        -- UNIQUE(tenant_id, codigo) + ON CONFLICT.
        INSERT INTO cuentas_contables (tenant_id, codigo, nombre, tipo, naturaleza)
        SELECT t.rnc, c.codigo, c.nombre, c.tipo, c.naturaleza
        FROM tenants t
        CROSS JOIN (VALUES
            ('1100', 'Caja y Bancos', 'ACTIVO', 'DEUDORA'),
            ('1110', 'Cuentas por Cobrar', 'ACTIVO', 'DEUDORA'),
            ('1150', 'ITBIS Adelantado', 'ACTIVO', 'DEUDORA'),
            ('1160', 'Depósitos Bancarios', 'ACTIVO', 'DEUDORA'),
            ('1200', 'Inventario', 'ACTIVO', 'DEUDORA'),
            ('1300', 'Anticipos a Empleados', 'ACTIVO', 'DEUDORA'),
            ('2100', 'ITBIS por Pagar', 'PASIVO', 'ACREEDORA'),
            ('2110', 'Cuentas por Pagar', 'PASIVO', 'ACREEDORA'),
            ('2200', 'Retenciones y Descuentos', 'PASIVO', 'ACREEDORA'),
            ('4100', 'Ingresos por Ventas', 'INGRESO', 'ACREEDORA'),
            ('4200', 'Otros Ingresos', 'INGRESO', 'ACREEDORA'),
            ('5050', 'Costo de Ventas', 'GASTO', 'DEUDORA'),
            ('5100', 'Gasto de Nómina', 'GASTO', 'DEUDORA'),
            ('5210', 'Gasto de Alquiler', 'GASTO', 'DEUDORA'),
            ('5220', 'Gasto de Servicios', 'GASTO', 'DEUDORA'),
            ('5230', 'Gasto de Transporte', 'GASTO', 'DEUDORA'),
            ('5290', 'Otros Gastos Operativos', 'GASTO', 'DEUDORA'),
            ('5295', 'Ajuste de Inventario (Merma)', 'GASTO', 'DEUDORA')
        ) AS c(codigo, nombre, tipo, naturaleza)
        ON CONFLICT (tenant_id, codigo) DO NOTHING;

        -- Agrupa las líneas de asientos_contables (sin tocar su forma
        -- actual) bajo una cabecera de asiento. Nullable por ahora; se
        -- vuelve NOT NULL en una migración posterior una vez verificado
        -- el backfill de abajo.
        ALTER TABLE asientos_contables ADD COLUMN IF NOT EXISTS asiento_id UUID REFERENCES asientos(id) ON DELETE CASCADE;
        CREATE INDEX IF NOT EXISTS idx_asientos_contables_asiento ON asientos_contables(asiento_id);
        CREATE INDEX IF NOT EXISTS idx_asientos_contables_cuenta_fecha ON asientos_contables(tenant_id, cuenta, fecha, created_at);

        -- Backfill 1/2: una cabecera por (tenant_id, referencia_tipo,
        -- referencia_id) ya generado por sincronizar - así es exactamente
        -- cómo esas líneas se insertaron siempre (una transacción, una
        -- referencia compartida).
        INSERT INTO asientos (tenant_id, fecha, descripcion, origen, referencia_tipo, referencia_id, usuario_id, created_at)
        SELECT DISTINCT ON (tenant_id, referencia_tipo, referencia_id)
            tenant_id, fecha, descripcion, 'AUTOMATICO', referencia_tipo, referencia_id, usuario_id, created_at
        FROM asientos_contables
        WHERE asiento_id IS NULL AND referencia_id IS NOT NULL
        ORDER BY tenant_id, referencia_tipo, referencia_id, created_at
        ON CONFLICT (tenant_id, referencia_tipo, referencia_id) DO NOTHING;

        UPDATE asientos_contables ac
        SET asiento_id = a.id
        FROM asientos a
        WHERE ac.asiento_id IS NULL
          AND ac.referencia_id IS NOT NULL
          AND a.tenant_id = ac.tenant_id
          AND a.referencia_tipo = ac.referencia_tipo
          AND a.referencia_id = ac.referencia_id;

        -- Backfill 2/2: mejor esfuerzo para asientos manuales pre-existentes
        -- (no tienen id compartido histórico) - agrupa por
        -- descripcion+fecha+usuario+segundo de creación, que es como
        -- create_asiento_manual insertaba sus líneas antes de esta
        -- migración (todas en la misma transacción, mismo instante).
        WITH grupos AS (
            SELECT tenant_id, descripcion, fecha, usuario_id, date_trunc('second', created_at) AS ts_sec,
                   MIN(created_at) AS created_at
            FROM asientos_contables
            WHERE asiento_id IS NULL
            GROUP BY tenant_id, descripcion, fecha, usuario_id, date_trunc('second', created_at)
        ),
        nuevos AS (
            INSERT INTO asientos (tenant_id, fecha, descripcion, origen, referencia_tipo, referencia_id, usuario_id, created_at)
            SELECT tenant_id, fecha, descripcion, 'MANUAL', 'MANUAL', NULL, usuario_id, created_at
            FROM grupos
            RETURNING id, tenant_id, fecha, descripcion, usuario_id, created_at
        )
        UPDATE asientos_contables ac
        SET asiento_id = n.id
        FROM nuevos n
        WHERE ac.asiento_id IS NULL
          AND ac.tenant_id = n.tenant_id
          AND ac.descripcion = n.descripcion
          AND ac.fecha = n.fecha
          AND (ac.usuario_id = n.usuario_id OR (ac.usuario_id IS NULL AND n.usuario_id IS NULL))
          AND date_trunc('second', ac.created_at) = date_trunc('second', n.created_at);

        -- Costo unitario del producto al momento de la venta (para el
        -- asiento de Costo de Ventas) - espeja compra_items.costo_unitario.
        -- Nullable: filas históricas no lo tienen y sincronizar las salta.
        ALTER TABLE venta_items ADD COLUMN IF NOT EXISTS costo_unitario DECIMAL(12,2);

        -- MODULO 15: Service Operations (Órdenes de Servicio / Work Orders).
        -- Entidad nueva y separada de `conduces` - conduces vuelve a ser
        -- solo la guía de despacho para todo tipo de tenant (ver revert en
        -- el frontend); esta es la orden de trabajo real con técnico,
        -- materiales, ciclo de vida y facturación. No hay columna `numero`:
        -- igual que ventas/compras/cotizaciones, se identifica por `id` (la
        -- UI puede mostrar un código corto derivado del id, como ya hacen
        -- los conduces con su prefijo CND-/OS- al imprimir).

        CREATE TABLE IF NOT EXISTS condiciones_orden (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            codigo TEXT NOT NULL,
            nombre TEXT NOT NULL,
            orden INT NOT NULL DEFAULT 0,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(tenant_id, codigo)
        );
        CREATE INDEX IF NOT EXISTS idx_condiciones_orden_tenant ON condiciones_orden(tenant_id);

        -- Backfill para tenants existentes; los tenants nuevos se siembran
        -- en auth_service::register (mismo patrón que cuentas_contables).
        INSERT INTO condiciones_orden (tenant_id, codigo, nombre, orden)
        SELECT t.rnc, c.codigo, c.nombre, c.orden FROM tenants t
        CROSS JOIN (VALUES
            ('MANTENIMIENTO', 'Mantenimiento', 10),
            ('REPARACION', 'Reparación', 20),
            ('GARANTIA', 'Garantía', 30),
            ('INSTALACION', 'Instalación', 40),
            ('INSPECCION', 'Inspección', 50),
            ('EMERGENCIA', 'Emergencia', 60)
        ) AS c(codigo, nombre, orden)
        ON CONFLICT (tenant_id, codigo) DO NOTHING;

        CREATE TABLE IF NOT EXISTS ordenes_servicio (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            cliente_id UUID REFERENCES clientes(id) ON DELETE SET NULL,
            cotizacion_id UUID REFERENCES cotizaciones(id),
            venta_id UUID REFERENCES ventas(id),
            condicion_id UUID REFERENCES condiciones_orden(id),
            estado TEXT NOT NULL DEFAULT 'BORRADOR', -- BORRADOR | PROGRAMADA | EN_PROCESO | PAUSADA | COMPLETADA | CANCELADA
            prioridad TEXT NOT NULL DEFAULT 'NORMAL', -- BAJA | NORMAL | ALTA | URGENTE
            fecha DATE NOT NULL DEFAULT CURRENT_DATE,
            fecha_programada DATE,
            hora_inicio TIME,
            hora_fin TIME,
            direccion TEXT,
            descripcion TEXT,
            subtotal DECIMAL(12,2) NOT NULL DEFAULT 0,
            descuento DECIMAL(12,2) NOT NULL DEFAULT 0,
            itbis_total DECIMAL(12,2) NOT NULL DEFAULT 0,
            total DECIMAL(12,2) NOT NULL DEFAULT 0,
            notas TEXT,
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_ordenes_servicio_tenant ON ordenes_servicio(tenant_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_ordenes_servicio_cliente ON ordenes_servicio(cliente_id);
        CREATE INDEX IF NOT EXISTS idx_ordenes_servicio_estado ON ordenes_servicio(tenant_id, estado);
        DO $$ BEGIN
            ALTER TABLE ordenes_servicio ADD CONSTRAINT chk_ordenes_servicio_estado
                CHECK (estado IN ('BORRADOR', 'PROGRAMADA', 'EN_PROCESO', 'PAUSADA', 'COMPLETADA', 'CANCELADA'));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;
        DO $$ BEGIN
            ALTER TABLE ordenes_servicio ADD CONSTRAINT chk_ordenes_servicio_prioridad
                CHECK (prioridad IN ('BAJA', 'NORMAL', 'ALTA', 'URGENTE'));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;

        CREATE TABLE IF NOT EXISTS orden_servicio_items (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            orden_servicio_id UUID NOT NULL REFERENCES ordenes_servicio(id) ON DELETE CASCADE,
            producto_id UUID NOT NULL REFERENCES productos(id),
            sku TEXT NOT NULL,
            nombre TEXT NOT NULL,
            tipo TEXT NOT NULL, -- PRODUCTO | SERVICIO (copiado de productos.tipo al insertar)
            cantidad DECIMAL(12,2) NOT NULL,
            precio_unitario DECIMAL(12,2) NOT NULL,
            descuento DECIMAL(12,2) NOT NULL DEFAULT 0,
            itbis_tipo TEXT NOT NULL,
            itbis_monto DECIMAL(12,2) NOT NULL,
            subtotal DECIMAL(12,2) NOT NULL,
            tecnico_id UUID REFERENCES empleados(id),
            observaciones TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_orden_servicio_items_orden ON orden_servicio_items(orden_servicio_id);

        CREATE TABLE IF NOT EXISTS orden_servicio_tecnicos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            orden_servicio_id UUID NOT NULL REFERENCES ordenes_servicio(id) ON DELETE CASCADE,
            empleado_id UUID NOT NULL REFERENCES empleados(id),
            rol TEXT NOT NULL DEFAULT 'TECNICO_PRINCIPAL', -- TECNICO_PRINCIPAL | ASISTENTE
            fecha_asignacion TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            fecha_inicio TIMESTAMPTZ,
            fecha_fin TIMESTAMPTZ
        );
        CREATE INDEX IF NOT EXISTS idx_orden_servicio_tecnicos_orden ON orden_servicio_tecnicos(orden_servicio_id);
        DO $$ BEGIN
            ALTER TABLE orden_servicio_tecnicos ADD CONSTRAINT chk_orden_servicio_tecnicos_rol
                CHECK (rol IN ('TECNICO_PRINCIPAL', 'ASISTENTE'));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;

        CREATE TABLE IF NOT EXISTS orden_servicio_materiales (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            orden_servicio_id UUID NOT NULL REFERENCES ordenes_servicio(id) ON DELETE CASCADE,
            producto_id UUID NOT NULL REFERENCES productos(id),
            cantidad_planificada DECIMAL(12,2) NOT NULL DEFAULT 0,
            cantidad_utilizada DECIMAL(12,2) NOT NULL DEFAULT 0,
            costo_unitario DECIMAL(12,2)
        );
        CREATE INDEX IF NOT EXISTS idx_orden_servicio_materiales_orden ON orden_servicio_materiales(orden_servicio_id);

        CREATE TABLE IF NOT EXISTS orden_servicio_notas (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            orden_servicio_id UUID NOT NULL REFERENCES ordenes_servicio(id) ON DELETE CASCADE,
            tipo TEXT NOT NULL DEFAULT 'INTERNA', -- INTERNA | TECNICO | CLIENTE | SISTEMA
            contenido TEXT NOT NULL,
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_orden_servicio_notas_orden ON orden_servicio_notas(orden_servicio_id, created_at DESC);
        DO $$ BEGIN
            ALTER TABLE orden_servicio_notas ADD CONSTRAINT chk_orden_servicio_notas_tipo
                CHECK (tipo IN ('INTERNA', 'TECNICO', 'CLIENTE', 'SISTEMA'));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;

        -- Los permisos `ordenes_servicio.gestionar`/`ordenes_compra.gestionar`
        -- que gobiernan las rutas nuevas de este módulo se agregan más abajo,
        -- junto con el resto del catálogo global de roles/permisos (ver
        -- MODULO 17 más adelante en este mismo archivo) - no hay un catálogo
        -- separado aquí, para no terminar con dos sistemas de permisos.

        -- Órdenes de compra reales (intención pre-recepción): `compras` sigue
        -- representando exclusivamente una compra YA recibida/pagada (sin
        -- estado, ver compras_service::create_compra) - recibir una orden de
        -- compra es lo que efectivamente crea la fila en `compras` vía el
        -- servicio existente, esta tabla nunca toca inventario directamente.
        CREATE TABLE IF NOT EXISTS ordenes_compra (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            proveedor_id UUID NOT NULL REFERENCES proveedores(id),
            orden_servicio_id UUID REFERENCES ordenes_servicio(id),
            estado TEXT NOT NULL DEFAULT 'BORRADOR', -- BORRADOR | ENVIADA | RECIBIDA_PARCIAL | RECIBIDA | CANCELADA
            subtotal DECIMAL(12,2) NOT NULL DEFAULT 0,
            itbis_total DECIMAL(12,2) NOT NULL DEFAULT 0,
            total DECIMAL(12,2) NOT NULL DEFAULT 0,
            fecha DATE NOT NULL DEFAULT CURRENT_DATE,
            fecha_esperada DATE,
            notas TEXT,
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_ordenes_compra_tenant ON ordenes_compra(tenant_id, created_at DESC);
        DO $$ BEGIN
            ALTER TABLE ordenes_compra ADD CONSTRAINT chk_ordenes_compra_estado
                CHECK (estado IN ('BORRADOR', 'ENVIADA', 'RECIBIDA_PARCIAL', 'RECIBIDA', 'CANCELADA'));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;

        CREATE TABLE IF NOT EXISTS orden_compra_items (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            orden_compra_id UUID NOT NULL REFERENCES ordenes_compra(id) ON DELETE CASCADE,
            producto_id UUID NOT NULL REFERENCES productos(id),
            sku TEXT NOT NULL,
            nombre TEXT NOT NULL,
            cantidad_solicitada DECIMAL(12,2) NOT NULL,
            cantidad_recibida DECIMAL(12,2) NOT NULL DEFAULT 0,
            costo_unitario DECIMAL(12,2) NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_orden_compra_items_orden ON orden_compra_items(orden_compra_id);

        -- Adjuntos genéricos: no existía ninguna abstracción de storage
        -- reusable (solo el patrón de disco de image_service.rs para fotos
        -- de producto, y el patrón BYTEA-cifrado de certificados_dgii, este
        -- último específico a un archivo pequeño/singular/sensible). Se
        -- generaliza el patrón de disco: archivos bajo
        -- {UPLOADS_DIR}/{tenant_id}/adjuntos/{entidad_tipo}/{entidad_id}/,
        -- servidos por el mismo ServeDir de /uploads ya montado.
        CREATE TABLE IF NOT EXISTS adjuntos (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            entidad_tipo TEXT NOT NULL, -- ORDEN_SERVICIO | COTIZACION | VENTA | CLIENTE | ORDEN_COMPRA
            entidad_id UUID NOT NULL,
            nombre_archivo TEXT NOT NULL,
            storage_path TEXT NOT NULL,
            mime_type TEXT,
            tamano BIGINT,
            usuario_id UUID REFERENCES usuarios(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_adjuntos_entidad ON adjuntos(tenant_id, entidad_tipo, entidad_id);
        DO $$ BEGIN
            ALTER TABLE adjuntos ADD CONSTRAINT chk_adjuntos_entidad_tipo
                CHECK (entidad_tipo IN ('ORDEN_SERVICIO', 'COTIZACION', 'VENTA', 'CLIENTE', 'ORDEN_COMPRA'));
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;

        -- MODULO 14: Catálogo de módulos vendibles + asignación por tenant.
        -- El catálogo es editable desde el sitio de staff (agregar/renombrar
        -- un módulo no necesita migración) - pero conectar un módulo nuevo a
        -- rutas reales todavía requiere un cambio de código en
        -- `required_modulo` (main.rs), igual que cualquier ruta nueva con
        -- `required_roles`. Ver docs del sitio de staff.
        CREATE TABLE IF NOT EXISTS modulos_catalogo (
            codigo TEXT PRIMARY KEY,   -- POS_VENTAS, INVENTARIO, ...
            nombre TEXT NOT NULL,
            descripcion TEXT,
            orden INT NOT NULL DEFAULT 0,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS tenant_modulos (
            tenant_id TEXT NOT NULL REFERENCES tenants(rnc) ON DELETE CASCADE,
            modulo_codigo TEXT NOT NULL REFERENCES modulos_catalogo(codigo) ON DELETE CASCADE,
            activado_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            activado_por TEXT, -- identificador libre de qué miembro del staff lo activó (sin login de staff individual todavía)
            PRIMARY KEY (tenant_id, modulo_codigo)
        );
        CREATE INDEX IF NOT EXISTS idx_tenant_modulos_tenant ON tenant_modulos(tenant_id);

        INSERT INTO modulos_catalogo (codigo, nombre, descripcion, orden) VALUES
            ('POS_VENTAS', 'Punto de Venta', 'Terminal de ventas, cotizaciones, conduces y clientes a crédito (fiado).', 10),
            ('INVENTARIO', 'Inventario', 'Productos, categorías, kardex y ajustes de stock.', 20),
            ('COMPRAS_GASTOS', 'Compras y Gastos', 'Compras a proveedores y gastos operativos.', 30),
            ('ORDENES_SERVICIO', 'Órdenes de Servicio', 'Órdenes de trabajo, técnicos, materiales y facturación de servicios.', 35),
            ('CONTABILIDAD', 'Contabilidad', 'Libro diario, libro mayor y períodos contables.', 40),
            ('CAJA_BANCOS', 'Caja y Bancos', 'Apertura/cierre de caja y cuentas bancarias.', 50),
            ('NOMINA', 'Nómina', 'Empleados, nómina y adelantos de sueldo.', 60),
            ('REPORTES', 'Reportes', 'Reportes DGII 606, financieros, de inventario y de ventas.', 70),
            ('DGII_ECF', 'Facturación Electrónica DGII', 'Firma y envío de e-CF, secuencias NCF y certificado.', 80),
            ('MOVIL', 'App Móvil', 'Acceso a la app móvil para POS y adelantos.', 90),
            ('IA_ASISTENTE', 'Asistente IA', 'Resumen del día y chat con IA.', 100)
        ON CONFLICT (codigo) DO NOTHING;

        -- Formato 606 (DGII, Norma 07-2018, instructivo vigente) tiene 23
        -- columnas; `compras` solo cubría 5. Estas columnas cierran esa
        -- brecha para poder generar el 606 completo sin inventar datos.
        -- Todas son nullable o tienen default porque las compras existentes
        -- no las tienen — se completan hacia adelante.
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS estado TEXT NOT NULL DEFAULT 'COMPLETADA'; -- COMPLETADA | ANULADA
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS tipo_documento TEXT NOT NULL DEFAULT 'FACTURA'; -- FACTURA | NOTA_CREDITO | NOTA_DEBITO
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS ncf_modificado TEXT; -- NCF original afectado por NC/ND (columna 4 del 606)
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS tipo_bienes_servicios SMALLINT; -- 1-11, columna 3 del 606
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS fecha_pago TIMESTAMPTZ;
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS monto_facturado_servicios DECIMAL(12,2) NOT NULL DEFAULT 0;
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS monto_facturado_bienes DECIMAL(12,2) NOT NULL DEFAULT 0;
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS itbis_retenido DECIMAL(12,2) NOT NULL DEFAULT 0;
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS itbis_proporcionalidad DECIMAL(12,2) NOT NULL DEFAULT 0;
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS itbis_costo DECIMAL(12,2) NOT NULL DEFAULT 0;
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS tipo_retencion_isr SMALLINT; -- 1-9, columna 17 del 606
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS monto_retencion_renta DECIMAL(12,2) NOT NULL DEFAULT 0;
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS isc DECIMAL(12,2) NOT NULL DEFAULT 0; -- Impuesto Selectivo al Consumo
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS otros_impuestos DECIMAL(12,2) NOT NULL DEFAULT 0;
        ALTER TABLE compras ADD COLUMN IF NOT EXISTS propina_legal DECIMAL(12,2) NOT NULL DEFAULT 0;

        -- MODULO 17: Permisos + roles - reemplaza usuarios.rol (texto fijo,
        -- 4 valores hardcodeados) como fuente real de autorización.
        -- usuarios.rol se conserva como etiqueta legible y fallback de
        -- migración (ver permission_guard en main.rs) - no se borra.
        --
        -- Catálogo GLOBAL (sin tenant_id), igual que modulos_catalogo:
        -- todos los tenants comparten el mismo catálogo de roles. Conectar
        -- un permiso nuevo a una ruta real todavía requiere un cambio en
        -- `required_permiso` (main.rs), igual que `required_modulo`.
        CREATE TABLE IF NOT EXISTS permisos_catalogo (
            codigo TEXT PRIMARY KEY,   -- ventas.gestionar, conduces.crear_retroactivo, ...
            nombre TEXT NOT NULL,
            descripcion TEXT,
            orden INT NOT NULL DEFAULT 0,
            activo BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS roles (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            codigo TEXT NOT NULL UNIQUE,  -- ADMIN, CAJERO, ALMACEN, CONTADOR, + los que cree el staff
            nombre TEXT NOT NULL,
            es_admin BOOLEAN NOT NULL DEFAULT false, -- bypass total, igual al "ADMIN always passes" de hoy
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS role_permisos (
            role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
            permiso_codigo TEXT NOT NULL REFERENCES permisos_catalogo(codigo) ON DELETE CASCADE,
            PRIMARY KEY (role_id, permiso_codigo)
        );

        ALTER TABLE usuarios ADD COLUMN IF NOT EXISTS rol_id UUID REFERENCES roles(id);
        ALTER TABLE usuarios ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT false;
        CREATE INDEX IF NOT EXISTS idx_usuarios_rol_id ON usuarios(rol_id);

        INSERT INTO permisos_catalogo (codigo, nombre, orden) VALUES
            ('productos.ver', 'Ver productos y categorías', 10),
            ('productos.editar', 'Editar productos y categorías', 11),
            ('ventas.gestionar', 'Punto de venta y ventas', 20),
            ('notas_credito.gestionar', 'Notas de crédito', 21),
            ('caja.gestionar', 'Caja', 22),
            ('clientes.gestionar', 'Clientes', 23),
            ('cotizaciones.gestionar', 'Cotizaciones', 24),
            ('conduces.gestionar', 'Conduces (entrega diferida)', 25),
            ('conduces.crear_retroactivo', 'Generar conduce de una venta ya completada (cuando no se marcó entrega diferida)', 26),
            ('inventario.gestionar', 'Inventario / kardex', 30),
            ('compras.gestionar', 'Compras', 31),
            ('proveedores.gestionar', 'Proveedores', 32),
            ('contabilidad.gestionar', 'Contabilidad', 40),
            ('bancos.gestionar', 'Bancos', 41),
            ('gastos.gestionar', 'Gastos', 42),
            ('reportes.dgii', 'Reportes DGII (606, IT-1)', 43),
            ('auditoria.ver', 'Auditoría', 44),
            ('nomina.gestionar', 'Empleados, nómina y adelantos', 50),
            ('config.gestionar', 'Configuración de la empresa', 60),
            ('tenants.ver', 'Ver datos del propio negocio', 61),
            ('backup.descargar', 'Descargar respaldo', 62),
            ('ecf.documentos', 'Documentos e-CF emitidos', 70),
            ('ecf.dev_tools', 'Herramientas internas de firma/pruebas e-CF', 71),
            ('ordenes_servicio.gestionar', 'Órdenes de servicio (trabajos, técnicos, materiales, condiciones)', 27),
            ('ordenes_compra.gestionar', 'Órdenes de compra', 33)
        ON CONFLICT (codigo) DO NOTHING;

        INSERT INTO roles (codigo, nombre, es_admin) VALUES
            ('ADMIN', 'Administrador', true),
            ('CAJERO', 'Cajero', false),
            ('ALMACEN', 'Almacén', false),
            ('CONTADOR', 'Contador', false)
        ON CONFLICT (codigo) DO NOTHING;

        -- Reproduce el acceso EFECTIVO de hoy exactamente (ver required_roles
        -- en main.rs) - nadie pierde ni gana acceso el día que esto corre.
        INSERT INTO role_permisos (role_id, permiso_codigo)
        SELECT r.id, seed.permiso_codigo FROM roles r
        JOIN (VALUES
            ('CAJERO', 'productos.ver'), ('CAJERO', 'ventas.gestionar'), ('CAJERO', 'notas_credito.gestionar'),
            ('CAJERO', 'caja.gestionar'), ('CAJERO', 'clientes.gestionar'), ('CAJERO', 'cotizaciones.gestionar'),
            ('CAJERO', 'conduces.gestionar'), ('CAJERO', 'ordenes_servicio.gestionar'),
            ('ALMACEN', 'productos.ver'), ('ALMACEN', 'productos.editar'), ('ALMACEN', 'inventario.gestionar'),
            ('ALMACEN', 'compras.gestionar'), ('ALMACEN', 'proveedores.gestionar'), ('ALMACEN', 'ordenes_compra.gestionar'),
            ('CONTADOR', 'contabilidad.gestionar'), ('CONTADOR', 'bancos.gestionar'), ('CONTADOR', 'gastos.gestionar'),
            ('CONTADOR', 'reportes.dgii'), ('CONTADOR', 'auditoria.ver'), ('CONTADOR', 'ecf.documentos')
        ) AS seed(rol_codigo, permiso_codigo) ON seed.rol_codigo = r.codigo
        ON CONFLICT DO NOTHING;

        -- Backfill: cada usuario existente apunta al role_id que coincide
        -- con su rol TEXT actual - migración sin pérdida de acceso.
        UPDATE usuarios u SET rol_id = r.id FROM roles r WHERE r.codigo = u.rol AND u.rol_id IS NULL;

        -- Conduce retroactivo (Feature 1): distingue de un conduce normal de
        -- entrega diferida - ver conduce_service::create_conduce_retroactivo.
        ALTER TABLE conduces ADD COLUMN IF NOT EXISTS retroactivo BOOLEAN NOT NULL DEFAULT false;

        -- Backfill: cualquier tenant que nunca haya sido curado por staff
        -- (cero filas en tenant_modulos) arranca viendo todo el catálogo,
        -- igual que antes de que el guard dejara de saltarse el chequeo en
        -- trial. Un tenant con AL MENOS una fila ya fue curado por staff -
        -- el NOT EXISTS es por tenant, no por módulo, así que esto nunca
        -- vuelve a agregar un módulo que staff apagó a propósito.
        INSERT INTO tenant_modulos (tenant_id, modulo_codigo)
        SELECT t.rnc, m.codigo
        FROM tenants t
        CROSS JOIN modulos_catalogo m
        WHERE NOT EXISTS (SELECT 1 FROM tenant_modulos tm WHERE tm.tenant_id = t.rnc)
        ON CONFLICT DO NOTHING;
        "#
    )
    .execute(&pool)
    .await?;

    println!("Migrations OK - EventStore + read models created");
    Ok(())
}
