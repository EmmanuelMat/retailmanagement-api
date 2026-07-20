# PLAN MAESTRO - SISTEMA COMPLETO COLMADO POS DOMINICANA
## Núcleo Bancario Rust + Next.js + DGII e-CF + Español 100%

> **Objetivo:** Sistema todo-en-uno para PYME Dominicana que reemplace Odoo/ERPNext con núcleo bancario real, no CRUD.

**Fecha:** 2026-07-19 | **Repo:** EmmanuelMat/retailmanagement-api | **Idioma UI:** Español Dominicano 100%

---

### VISIÓN GENERAL

Un colmado en Santo Domingo necesita:
1. **Vender rápido** (POS) con impresora térmica 80mm
2. **Facturar legal** DGII e-CF E31/E32, RFCE diario, 606/607/608, IT-1
3. **Controlar inventario** que no quede negativo nunca (TigerBeetle `debits_must_not_exceed_credits`)
4. **Saber Ganancias** contabilidad real double-entry, no Excel
5. **Pagar nómina** con adelantos 50% sin enredos (EWA dominicano)
6. **Cuadrar caja** apertura/cierre, bancos, conciliación
7. **Trabajar sin internet** (Tauri + SQLite queue)

**Arquitectura:** Event Sourcing (Postgres tabla `events` apéndice, hash encadenado) + Ledger TigerBeetle (dinero) + Read Models (Prisma) + Rust core (firma XAdES, DGII real) + Next.js (BFF Español) + Expo Móvil.

---

### MÓDULOS - 14 Módulos, Plan Modular

Cada módulo tiene: Objetivo Español + Submódulos + Entidades DB + Eventos + API Rust (gRPC/HTTP) + UI Next.js (Español) + Ledger + DGII

#### MÓDULO 1: AUTENTICACIÓN Y MULTI-TENANCY (Base)

**Objetivo:** Cada negocio (RNC) aislado, usuarios con roles, JWT tenant_id = RNC.

**Submódulos:**
- Registro negocio (RNC, Razón Social, Certificado P12 DGII)
- Usuarios: Admin (dueño), Cajero (POS), Almacén (inventario), Contador (solo reportes)
- Sesiones, JWT, RLS (Row Level Security)

**Entidades:**
```sql
Tenant { id (RNC PK), razonSocial, direccion, telefono, logoUrl, rnc, ncfSecuencias (JSON), p12EncryptedUrl, p12PasswordEncrypted, dgiiAmbiente: TesteCF/CerteCF/eCF, createdAt }
Usuario { id, tenantId (RNC), nombre, email, passwordHash, rol: ADMIN/CAJERO/ALMACEN/CONTADOR, activo, ultimoAcceso }
Sesion { id, usuarioId, token, expira }
```

**Eventos:**
- `TenantRegistrado`, `TenantCertificadoSubido`, `UsuarioCreado`, `UsuarioRolCambiado`, `SesionIniciada`

**API Rust (Falta - PLAN):**
- `POST /v1/auth/register` - Crea tenant + admin
- `POST /v1/auth/login` - JWT con tenant_id
- `POST /v1/tenants/:rnc/certificado` - Sube P12 encriptado S3
- `GET /v1/tenants/:rnc/secuencias` - Lista secuencias DGII autorizadas E31/E32
- `POST /v1/tenants/:rnc/secuencias/solicitar` - Mock que simula OFV DGII para dev

**UI Español:**
- `app/(auth)/login/page.tsx` - "Iniciar Sesión • Colmado POS"
- `app/(auth)/registro/page.tsx` - "Registrar Mi Negocio • RNC, Certificado DGII"
- `app/(dashboard)/configuracion/usuarios/page.tsx` - "Usuarios y Roles • Cajeros, Almacén"
- `app/(dashboard)/configuracion/empresa/page.tsx` - "Mi Empresa • RNC, Logo, Dirección"

**Ledger:** No aplica.

**Falta en API actual:** Todo este módulo falta. Actualmente tenemos mock tenant 130793752. Plan: Implementar `auth_service.rs` en Rust core + `event_store` TenantAggregate + `read_tenants` projector.

---

#### MÓDULO 2: PRODUCTOS Y CATEGORÍAS

**Objetivo:** Catálogo con ITBIS por producto (18%, 16% reducida, Exento), código barras, costo promedio, precio.

**Submódulos:**
- Categorías: Víveres, Bebidas Fríos, Panadería, Limpieza, Enlatados, Dulces (custom por negocio)
- Productos: CRUD, foto, SKU, código barras, unidad medida DGII (43=unidad, 17=libra, etc), ITBIS tipo, costo, precio, stock mínimo alerta
- Variantes: Plátanos verde/maduro (opcional)

**Entidades:**
```sql
Categoria { id, tenantId, nombre (VÍVERES), color (amber-400), icono (🌽), orden }
Producto { id, tenantId, categoriaId, sku (PLT-001), codigoBarras, nombre (PLÁTANOS X LIBRA), descripcion, unidadMedida DGII, itbisTipo: GRAVA_18/GRAVA_16/EXENTO, costoPromedio, precioVenta, precioMayor, stockMinimo, activo, fotoUrl }
ProductoVariante { id, productoId, nombre (Verde), sku, precio }
```

**Eventos:**
- `CategoriaCreada`, `ProductoCreado`, `ProductoPrecioCambiado`, `ProductoCostoActualizado (por compra)`, `ProductoStockMinimoCambiado`

**API Rust (Falta):**
- `POST /v1/productos` - Crea producto + evento ProductoCreado + crea cuenta TigerBeetle asset:inventario:{sku}
- `GET /v1/productos?tenantId=...&categoriaId=...&search=...` - Desde read_model read_inventory_stock
- `PUT /v1/productos/:id` - Update
- `POST /v1/categorias` - Crea categoria
- `GET /v1/categorias`

**UI Español:**
- `app/(dashboard)/inventario/categorias/page.tsx` - "Categorías • Víveres, Bebidas..."
- `app/(dashboard)/inventario/productos/page.tsx` - "Productos • Catálogo con ITBIS 18%/16%/Exento" - Tabla con foto, SKU, stock, costo, precio, ITBIS badge, acciones Editar/Eliminar
- `app/(dashboard)/inventario/productos/nuevo/page.tsx` - Formulario: Nombre, SKU, Categoría, Unidad Medida DGII (43 unidad), ITBIS (18/16/Exento), Costo, Precio, Stock mínimo, Código barras
- `app/(dashboard)/inventario/productos/[id]/page.tsx` - Detalle + historial movimientos + kardex

**Ledger:** Cada producto tiene cuenta TigerBeetle `asset:inventario:{sku}` con `credits_posted` = stock valorizado costo promedio. Al recibir compra, debit inventario.

**DGII:** `UnidadMedida` y `IndicadorBienoServicio` (1=bien, 2=servicio) y `IndicadorFacturacion` (1=gravado, 2=exento) deben mapearse a producto.itbisTipo para e-CF DetallesItems.

**Falta:** Todo CRUD productos. Actualmente solo mock array en POS UI.

---

#### MÓDULO 3: INVENTARIO Y ALMACÉN

**Objetivo:** Kardex, movimientos, reservas (pending) -> commit, nunca negativo, multi-almacén opcional.

**Submódulos:**
- Stock actual, reservado, disponible = onHand - reserved
- Entradas: Compras proveedor, ajustes inventario
- Salidas: Ventas POS, mermas, vencidos
- Traslados entre almacenes
- Inventario físico y ajustes

**Entidades:**
```sql
Almacen { id, tenantId, nombre (Principal, Almacén 2), direccion }
InventarioMovimiento { id, tenantId, productoId, almacenId, tipo: ENTRADA_COMPRA/SALIDA_VENTA/AJUSTE/TRASLADO/RESERVA, cantidad, costoUnitario, referenciaId (ventaId o compraId), createdAt }
InventarioReserva { id, productoId, cantidad, saleId, estado: PENDING/POSTED/VOIDED, tbTransferId }
InventarioFisico { id, almacenId, fecha, estado: ABIERTO/CERRADO, lineas: [{productoId, stockSistema, stockFisico, diferencia}] }
```

**Eventos:**
- `StockRecibido { sku, qty, costo }`, `StockReservado { sku, qty, saleId, tbTransferId pending }`, `StockComprometido { saleId, tb post_pending }`, `StockLiberado { saleId, tb void_pending }`, `StockAjustado`, `TrasladoIniciado`

**API Rust (Falta):**
- `POST /v1/inventario/reservar` - Crea pending transfer en TigerBeetle: debit asset:inventario, credit liability:reserva_pendiente (flag pending), si stock insuficiente TB reject con `exceeds_credits`
- `POST /v1/inventario/comprometer` - post_pending
- `POST /v1/inventario/liberar` - void_pending
- `POST /v1/inventario/entrada` - Compra: aumenta stock + actualiza costo promedio `(stock*qty + newQty*newCost)/(totalQty)`
- `GET /v1/inventario/stock?sku=...` - Desde read_inventory_stock
- `POST /v1/inventario/fisico/iniciar` - Crea inventario físico

**UI Español:**
- `app/(dashboard)/inventario/page.tsx` - Dashboard: Stock valorizado total, productos bajo mínimo (alerta), movimientos hoy, gráfico kardex
- `app/(dashboard)/inventario/movimientos/page.tsx` - "Movimientos • Kardex" Tabla: Fecha, Producto, Tipo, Cantidad, Costo, Referencia, Usuario
- `app/(dashboard)/inventario/fisico/page.tsx` - "Inventario Físico • Conteo" - Crear conteo, escanear barras, comparar sistema vs físico, generar ajuste
- `app/(dashboard)/inventario/ajustes/page.tsx` - "Ajustes • Mermas, Vencidos"

**Ledger:** 
- `asset:inventario:{sku}` - stock valorizado
- `liability:reserva_pendiente` - para reservas POS pendientes
- `expense:costo_venta` + `expense:merma`
- Transferencias vinculadas: Venta -> Commit stock + COGS

**Falta:** Todo. Actualmente solo mock reserva en ledger/mod.rs.

---

#### MÓDULO 4: CLIENTES Y PROVEEDORES

**Objetivo:** Clientes con RNC/Cédula validación DGII, límite crédito, balance. Proveedores con RNC, 606 auto.

**Entidades:**
```sql
Cliente { id, tenantId, tipo: CONSUMIDOR_FINAL/EMPRESA, rncOCedula (000000000 para final), nombre, telefono, correo, direccion, limiteCredito, balance, activo }
Proveedor { id, tenantId, rnc, nombre, telefono, correo, direccion, contacto, balancePorPagar, activo }
```

**Eventos:**
- `ClienteCreado`, `ClienteLimiteCreditoActualizado`, `ProveedorCreado`

**API Rust (Falta):**
- `POST /v1/clientes` - Valida RNC con DGII API (scrap o servicio consulta), crea cliente
- `GET /v1/clientes?search=...`
- `POST /v1/proveedores`
- `GET /v1/proveedores`
- `GET /v1/clientes/:id/balance` - Desde TigerBeetle asset:cuentas_por_cobrar:{clienteId}

**UI Español:**
- `app/(dashboard)/clientes/page.tsx` - "Clientes • Consumidor Final y Crédito Fiscal" - Tabla, buscar RNC, crear, ver balance, historial compras, estado cuenta
- `app/(dashboard)/clientes/nuevo/page.tsx` - Form: Tipo (Consumidor Final no necesita RNC si <250k, Empresa necesita RNC), RNC/Cédula, Nombre, Tel, Correo, Límite Crédito
- `app/(dashboard)/proveedores/page.tsx` - "Proveedores • Para 606" - Tabla RNC, balance por pagar, compras
- `app/(dashboard)/proveedores/nuevo/page.tsx`

**Ledger:**
- `asset:cuentas_por_cobrar:{clienteId}` - cuando vendes a crédito
- `liability:cuentas_por_pagar:{proveedorId}` - cuando compras

**DGII:** Validación RNC via `GET /consultadirectorio/api/consultas/obtenerdirectorioporrnc?rnc=` (existe) o scraping DGII RNC consulta. Cliente consumidor final RNC 000000000 no necesita validación. Cliente empresa RNC debe existir y estar activo.

---

#### MÓDULO 5: PUNTO DE VENTA (POS) - Corazón

**Objetivo:** Venta ultra-rápida, offline, con impresora térmica, QR DGII, pago múltiple, propina legal 10%.

**Submódulos:**
- Terminal táctil, búsqueda producto por nombre/SKU/barras
- Carrito, descuentos por línea y global, propina 10% opcional
- Cliente: consumidor final (default) o empresa (RNC para E31)
- Pagos: Efectivo (calcula devuelta), Tarjeta, Transferencia, Crédito (si cliente tiene límite)
- Facturación: Determina tipo e-CF automáticamente: Si cliente RNC válido y necesita crédito fiscal -> E31, si no -> E32. Si E32 y total >=250k -> requiere RNC/Cédula cliente.
- Impresión: Ticket 80mm con logo, productos, ITBIS desglosado, total, QR DGII, codigo seguridad, TrackID, pie "Gracias"
- Contingencia: Si no internet, vende offline, guarda eventos en IndexedDB + SQLite Tauri queue, marca IndicadorEnvioDiferido=1, sync cuando vuelve internet

**Entidades:**
```sql
Venta { id (aggregateId), tenantId, eNCF (E320000000001), tipoECF (31/32), clienteId, cajaId, estado: BORRADOR/COMPLETADA/DGII_PENDIENTE/DGII_ACEPTADA/DGII_RECHAZADA/ANULADA, subtotalGravado, subtotalExento, itbisTotal, descuentoGlobal, propina, montoTotal, formaPago: EFECTIVO/TARJETA/TRANSFER/CREDITO, montoEntregado, devuelta, codigoSeguridad, trackId, qrUrl, esContingencia, createdAt }
VentaItem { ventaId, productoId, cantidad, precioUnitario, descuento, itbis, montoItem }
Caja { id, tenantId, usuarioId, fechaApertura, montoApertura, estado: ABIERTA/CERRADA, fechaCierre, montoCierreSistema, montoCierreReal, diferencia }
```

**Eventos:**
- `VentaCreada`, `ItemAgregadoCarrito { sku, qty, precio }`, `DescuentoAplicado`, `ClienteAsignado { clienteId, esConsumidorFinal }`, `VentaCompletada { eNCF, total, itbis, tipoECF }`, `ETicketFirmaSolicitada { xml }`, `ETicketFirmado { codigoSeguridad, qrUrl }`, `ETicketEnviadoDGII { trackId }`, `ETicketAceptado { trackId, estado Aceptado }`, `ETicketRechazado { motivo }`, `VentaAnulada { motivo, NC eNCF }`, `CajaAbierta`, `CajaCerrada`

**API Rust (Parcialmente existe, falta completar):**
- Existe: `/v1/ecf/build`, `/build-sign`, `/build-sign-send` (construye XML per Informe Tecnico + firma XAdES + envía DGII real con seed auth -> TrackID poll)
- Falta:
  - `POST /v1/pos/ventas` - Crea venta aggregate, reserva stock TigerBeetle pending, crea eventos, llama ecf_builder + signer async via NATS, retorna QR inmediato
  - `POST /v1/caja/abrir` - Abre caja con monto inicial, evento CajaAbierta, TB transfer: debit caja, credit capital?
  - `POST /v1/caja/cerrar` - Cierra, calcula sistema vs real, evento
  - `GET /v1/pos/caja/:id/ventas` - Ventas de caja actual para cierre
  - `POST /v1/pos/devolucion` - Nota crédito E34 referenciando E31/E32 original via InformacionReferencia

**UI Español (Actual 1 página, expandir a módulos):**
- `app/(dashboard)/pos/page.tsx` - Ya existe con diseño único colmado hand-painted (actual). Mejorar con:
  - Panel izquierdo categorías, centro productos con tilt, derecha recibo papel zigzag (ya hecho)
  - Modal cliente: Buscar RNC, crear rápido
  - Modal pago: Efectivo con calculadora devuelta, Tarjeta con referencia, Crédito con validación límite
  - Teclado numérico táctil
  - Barra superior: Caja abierta/cerrada, hora, RNC, usuario
  - Footer: Métodos pago + botón Cobrar grande amarillo brutalist
- `app/(dashboard)/pos/caja/page.tsx` - "Caja • Apertura y Cierre" - Monto apertura, ventas del día (47 E32), ITBIS cobrado, efectivo vs tarjeta, diferencia, botón Cerrar Caja
- `app/(dashboard)/pos/historial/page.tsx` - "Ventas del Día • Historial" - Tabla ventas, eNCF, cliente, total, estado DGII (Aceptado/Pendiente/Rechazado), TrackID, reimprimir ticket

**Ledger:**
- Venta contado E32: 
  - Linked transfers atómicos (3):
    1. Debit asset:cuentas_por_cobrar:{clienteId o consumidor} 1180 / Credit revenue:ventas_gravadas_18 1000
    2. Debit asset:cuentas_por_cobrar / Credit liability:itbis_por_pagar 180
    3. Debit expense:costo_venta 600 / Credit asset:inventario:{sku} 600
  - Si efectivo: Debit asset:caja 1180 / Credit asset:cuentas_por_cobrar 1180
  - Si tarjeta: Debit asset:banco_tarjeta / Credit cuentas_por_cobrar

**DGII:**
- Lógica tipo e-CF: 
  - Si cliente tipo CONSUMIDOR_FINAL y total <250k -> E32 con RFCE (resumen diario, no envía individual a DGII, guarda y envía RFCE cierre)
  - Si cliente tipo EMPRESA con RNC válido -> E31
  - Si total E32 >=250k -> Requiere RNC/Cédula cliente, envía individual a DGII (no RFCE)
  - Si cliente no tiene RNC y total >=250k -> Bloquear venta, pedir RNC

**Falta:** Caja apertura/cierre UI, devoluciones E34, pago crédito validación límite, impresora térmica 80mm integration (Tauri print), offline IndexedDB queue.

---

#### MÓDULO 6: COMPRAS Y GASTOS (Para 606)

**Objetivo:** Registrar facturas de proveedores con NCF/B, validar eNCF vs DGII, generar 606 auto, actualizar inventario y costo promedio.

**Entidades:**
```sql
Compra { id, tenantId, proveedorId, ncf (B01...), eNCF (E31...), fecha, subtotal, itbis, total, formaPago, estado: PENDIENTE/PAGADA, archivoXmlUrl }
CompraItem { compraId, productoId, cantidad, costoUnitario, itbis }
GastoMenor { id, tenantId, concepto, monto, ncf B13/E43, empleadoId, reciboUrl }
```

**Eventos:**
- `CompraRegistrada { proveedorId, ncf, total, itbis }`, `CompraPagada`, `GastoMenorRegistrado`

**API Rust (Falta):**
- `POST /v1/compras` - Valida NCF via DGII consulta, crea compra, evento CompraRegistrada, actualiza inventario (entrada), TB: Debit inventario / Credit cuentas_por_pagar
- `POST /v1/compras/:id/pagar` - Paga: Debit cuentas_por_pagar / Credit banco/caja
- `GET /v1/compras?periodo=202607` - Para 606
- `POST /v1/gastos-menores` - B13/E43 para 606 gastos menores

**UI Español:**
- `app/(dashboard)/compras/page.tsx` - "Compras • Proveedores y 606" - Tabla facturas proveedor con NCF, RNC, total, ITBIS, estado, validar DGII badge (Aceptado), generar 606
- `app/(dashboard)/compras/nueva/page.tsx` - Form: Proveedor (buscar RNC), NCF (B01), Fecha, Productos (sku, qty, costo), Total, ITBIS auto, subir XML si es e-CF recibido
- `app/(dashboard)/gastos/page.tsx` - "Gastos Menores • B13/E43" - Empleado, concepto, monto, recibo

**Ledger:**
- Compra: Debit asset:inventario / Credit liability:cuentas_por_pagar:{proveedorId}
- Pago: Debit liability:cuentas_por_pagar / Credit asset:banco

**DGII:** Validar NCF recibido via `https://ecf.dgii.gov.do/eCF/ConsultaTimbre?...` o servicio Consulta Estado. Guardar XML para 10 años.

---

#### MÓDULO 7: CONTABILIDAD (Libro Mayor Real)

**Objetivo:** Contabilidad double-entry real, no solo reportes. Asientos automáticos desde ventas/compras/nómina, libro mayor, balanza, estado resultados.

**Entidades:**
```sql
CuentaContable { id, tenantId, codigo (1-01-001), nombre (Caja), tipo: ACTIVO/PASIVO/PATRIMONIO/INGRESO/COSTO/GASTO, nivel, padreId }
AsientoContable { id, tenantId, numero, fecha, concepto, referenciaTipo (VENTA/COMPRA/NOMINA/AJUSTE), referenciaId, totalDebe, totalHaber, estado: BORRADOR/POSTEADO }
AsientoLinea { asientoId, cuentaId, debe, haber, descripcion }
PeriodoContable { id, tenantId, periodo (202607), estado: ABIERTO/CERRADO, fechaCierre }
```

**Eventos:**
- `AsientoCreado`, `AsientoPosteado`, `PeriodoCerrado`

**API Rust (Falta):**
- `POST /v1/contabilidad/asientos` - Crea asiento desde evento (venta, compra, nómina usan linked transfers TigerBeetle ya, pero también crea asiento contable para reporte)
- `POST /v1/contabilidad/asientos/:id/postear` - Postea: valida debe==haber, cambia estado, TB ya posteó
- `GET /v1/contabilidad/libro-mayor?cuentaId=&periodo=` - Desde read_account_balances + asientos
- `GET /v1/contabilidad/balanza?periodo=` - Suma débitos/créditos por cuenta
- `POST /v1/contabilidad/cierre-periodo` - Cierra periodo, no permite más asientos en ese periodo

**UI Español:**
- `app/(dashboard)/contabilidad/page.tsx` - Dashboard contable: Balance general (Activos=Pasivos+Patrimonio), Estado Resultados (Ingresos-Costos=Ganancia), gráfico
- `app/(dashboard)/contabilidad/libro-mayor/page.tsx` - "Libro Mayor • TigerBeetle Vivo" - Filtro cuenta, periodo, tabla movimientos, balance acumulado
- `app/(dashboard)/contabilidad/asientos/page.tsx` - "Asientos Contables • Automáticos y Manuales" - Lista asientos, ver detalle débitos/créditos, estado
- `app/(dashboard)/contabilidad/periodos/page.tsx` - "Periodos • Cierre Mensual" - Cerrar mes, no se puede facturar en periodo cerrado

**Ledger:** Todo usa TigerBeetle ya, pero contabilidad es vista agrupada por cuenta contable mapeada a cuenta TigerBeetle. Ej: Cuenta contable 1-01-001 Caja -> tb account id asset:caja

**Falta:** Todo contabilidad avanzada. Actualmente solo ledger balances simples.

---

#### MÓDULO 8: NÓMINA Y ADELANTOS (EWA Dominicano) - Ya iniciado

**Objetivo:** Empleados, turnos, cálculo TSS, ISR, adelantos 50% con deducción automática quincena, préstamos.

**Submódulos:**
- Empleados: CRUD, salario, puesto, fecha ingreso, cuenta banco para pago
- Turnos/Asistencia: ClockIn/Out, horas acumuladas, comisiones por ventas
- Adelantos: Solicitud 50% sueldo ganado, aprobación gerente, TB pending->posted, deducción nómina
- Nómina: Cálculo quincenal/mensual, TSS (5.91% empleado: 2.87% AFP + 3.04% SCF), ISR progresivo DGII tablas, deducciones (adelantos, préstamos, cooperativa), neto a pagar, archivo banco para dispersión
- Préstamos: Empleado pide préstamo, cuotas mensuales descontadas nómina

**Entidades:**
```sql
Empleado { id, tenantId, nombre, cedula, puesto, salarioMensual, salarioPorHora, fechaIngreso, cuentaBanco, activo, fotoUrl }
Turno { id, empleadoId, fecha, horaEntrada, horaSalida, horasTrabajadas, ventaComision }
Adelanto { id, empleadoId, montoSolicitado, montoAprobado, razon, estado: SOLICITADO/APROBADO/RECHAZADO/DEDUCIDO, fechaSolicitud, fechaAprobacion, aprobadoPor, tbTransferId, payrollDeduccionId }
Prestamo { id, empleadoId, monto, tasaInteres, cuotasTotales, cuotaMensual, balancePendiente, estado }
NominaRun { id, tenantId, periodo (2026071=1ra quincena julio), fecha, totalBruto, totalTSS, totalISR, totalAdelantos, totalNeto, estado: BORRADOR/CERRADA/PAGADA }
NominaDetalle { nominaRunId, empleadoId, salarioBruto, horas, tss, isr, adelantoDeduccion, prestamoDeduccion, neto, estado }
```

**Eventos:**
- `EmpleadoCreado`, `TurnoRegistrado { horas }`, `HorasAcumuladas { amount }`, `AdelantoSolicitado`, `AdelantoAprobado { tbTransferId }`, `AdelantoRechazado`, `NominaIniciada { periodo }`, `NominaEmpleadoCalculado { gross, tss, isr, adelantoDeduccion, neto }`, `NominaCerrada`, `NominaPagada`

**API Rust (Parcial, falta):**
- Existe: `src/aggregates/employee.rs` con 50% regla, `payroll_service.rs` con reserve/post TB, `/v1/advances/request`, `/v1/advances/approve`, `/v1/employees/:id/balance`
- Falta:
  - `POST /v1/empleados` - Crea empleado
  - `POST /v1/empleados/:id/clock-in` - Registra turno, evento HoursAccrued, actualiza accrued_net read model
  - `GET /v1/empleados/:id/adelantos` - Historial adelantos
  - `POST /v1/nomina/run` - Ya existe mock, falta real: carga todos empleados, para cada uno calcula gross desde accrued_net + salario base, tss, isr tabla DGII, adelantoDeduccion = min(advance_balance, gross-tss-isr), net, crea linked transfers TigerBeetle 4 transfers atómicos por empleado (ver docs/05-PAYROLL-ADVANCE.md), evento NominaCerrada
  - `GET /v1/nomina/:id/detalles` - Detalle por empleado para impresión desprendible

**UI Español:**
- `app/(dashboard)/nomina/page.tsx` - Dashboard: Total nómina quincena, adelantos activos RD$6k, próximos pagos, empleados con más adelantos
- `app/(dashboard)/nomina/empleados/page.tsx` - "Empleados • Equipo" - Tabla foto, nombre, puesto, salario, balance adelantos, estado, acciones
- `app/(dashboard)/nomina/empleados/nuevo/page.tsx` - Form: Nombre, Cédula, Puesto, Salario Mensual, Por Hora, Cuenta Banco
- `app/(dashboard)/nomina/adelantos/page.tsx` - "Adelantos • EWA 50%" - Ya diseñado como post-it amarillo en POS, expandir a página completa con tabla solicitudes, filtros pendiente/aprobado/deducido, botón Aprobar/Rechazar, ver TB transferId
- `app/(dashboard)/nomina/turnos/page.tsx` - "Turnos y Asistencia • Horas Acumuladas" - Calendario, clock in/out, horas del mes, comisiones ventas
- `app/(dashboard)/nomina/run/page.tsx` - "Correr Nómina • Quincenal" - Seleccionar periodo (2026071), botón "Calcular Nómina" -> muestra tabla empleados con bruto, TSS, ISR, adelantos, neto, botón "Cerrar y Pagar" que hace TB linked transfers y genera asiento contable y archivo banco

**Ledger (Crítico):**
- Adelanto solicitado: TB reserve pending: Debit asset:anticipos_empleados / Credit asset:caja (pending) -> al aprobar post_pending
- Nómina run por empleado (linked atómico 4 transfers):
  1. Debit expense:sueldos Bruto / Credit liability:tss_por_pagar 5.91%
  2. Debit expense:sueldos / Credit liability:isr_retenido (tabla DGII)
  3. Debit expense:sueldos / Credit asset:anticipos_empleados adelantoDeduccion (settles advance)
  4. Debit expense:sueldos / Credit asset:banco neto a pagar

**DGII/TSS:** TSS 2.87% AFP + 3.04% SCF = 5.91% empleado, ISR progresivo: exento <416,220 anual, 15% hasta 624,329, 20% hasta 867,123, 25% >... (tabla 2024). Calcular per quincena.

**Falta:** Turnos, préstamos, cálculo ISR tabla real, archivo banco dispersión, desprendible PDF.

---

#### MÓDULO 9: CAJA Y BANCOS

**Objetivo:** Apertura, cierre, arqueo, conciliación bancaria, gastos caja chica.

**Entidades:**
```sql
Caja { id, tenantId, usuarioId, nombre (Caja Principal), estado: ABIERTA/CERRADA, montoApertura, fechaApertura, montoCierreSistema, montoCierreReal, diferencia, observaciones }
MovimientoCaja { id, cajaId, tipo: VENTA/COMPRA/GASTO/INGRESO/RETIRO, monto, referenciaId, descripcion, createdAt }
Banco { id, tenantId, nombre (Banco Popular), numeroCuenta, balance }
MovimientoBanco { id, bancoId, tipo, monto, referencia, fecha, conciliado: bool }
```

**Eventos:**
- `CajaAbierta { montoApertura }`, `MovimientoCajaRegistrado`, `CajaCerrada { montoSistema, montoReal, diferencia }`, `ArqueoRealizado`

**API Rust (Falta):**
- `POST /v1/caja/abrir` - Abre caja, TB: Debit caja / Credit capital? o solo evento?
- `POST /v1/caja/movimiento` - Ingreso/retiro: Debit/Credit caja vs gasto/ingreso
- `POST /v1/caja/cerrar` - Cierra, calcula ventas POS del día desde read_sales, compara con real, diferencia va a expense:sobrante/faltante
- `GET /v1/caja/:id/arqueo` - Arqueo: efectivo, tarjeta, transferencia por separado

**UI Español:**
- `app/(dashboard)/caja/page.tsx` - "Caja • Apertura, Movimientos, Cierre y Arqueo" - Estado actual Abierta/Cerrada, monto apertura, ventas hoy 47, efectivo RD$18,420, tarjeta, diferencia, botón Cerrar Caja con modal monto real contado
- `app/(dashboard)/caja/historial/page.tsx` - Historial cierres, ver diferencias, auditar
- `app/(dashboard)/bancos/page.tsx` - "Bancos • Cuentas y Conciliación" - Lista bancos, balances TigerBeetle, movimientos, conciliar con estado banco

**Ledger:**
- Apertura: No movimiento, solo evento
- Venta efectivo: Debit caja / Credit cuentas_por_cobrar
- Retiro dueño: Debit expense:retiro_dueno / Credit caja
- Cierre diferencia: Si sobra: Debit caja / Credit ingreso:sobrante_caja; Si falta: Debit gasto:faltante_caja / Credit caja

---

#### MÓDULO 10: REPORTES Y DASHBOARD

**Objetivo:** Dashboard gerencial con métricas reales desde TigerBeetle y EventStore, no mock.

**Submódulos:**
- Dashboard hoy: Ventas, ITBIS, Ticket promedio, productos más vendidos, empleados con más ventas, gráfico horas pico
- Reportes DGII: 606 Compras, 607 Ventas, 608 Anulados, 609 Pagos Exterior, IT-1 borrador, 623? etc
- Reportes gerenciales: Estado resultados, balance general, inventario valorizado, antigüedad CxC y CxP, nómina vs ventas
- Export: Excel, PDF, TXT DGII

**API Rust (Parcial):**
- Existe `/v1/reports/606`, `/607` mock
- Falta: Real que genera TXT desde read models + validación pre-validación DGII tool, IT-1 desde Totales, dashboard metrics desde TigerBeetle balances + read_sales aggregation via SQL

**UI Español:**
- `app/(dashboard)/page.tsx` - Dashboard principal (actualmente POS, mover POS a /pos, y dashboard general aquí) - Cards: Ventas hoy, ITBIS hoy, Ticket promedio, Top productos, Gráfico ventas por hora, Alertas stock bajo, Adelantos pendientes
- `app/(dashboard)/reportes/page.tsx` - "Reportes • Gerenciales y Fiscales"
- `app/(dashboard)/reportes/dgii/page.tsx` - "DGII • 606/607/608/IT-1" - Selector periodo AAAAMM, botones Exportar TXT pre-validado, ver errores, subir a OFV
- `app/(dashboard)/reportes/ventas/page.tsx` - "Ventas • Por Producto, Empleado, Hora"
- `app/(dashboard)/reportes/inventario/page.tsx` - "Inventario Valorizado • Kardex y Costo Promedio"

---

#### MÓDULO 11: CONFIGURACIÓN DGII Y EMPRESA

**Objetivo:** Todo lo que contador configura.

**Entidades:**
- ConfigDGII: RNC, secuencias E31/E32 autorizadas con fecha vencimiento, certificado P12 path, ambiente TesteCF/CerteCF/eCF, próximo e-NCF a usar por tipo
- ConfigEmpresa: Logo, dirección, teléfono, correo, pie factura, propina legal 10% activa, ITBIS incluido en precio?
- ConfigImpresora: Tipo (80mm térmica, PDF), IP impresora, plantilla ticket
- Secuencias: Tabla secuencias con tipo, desde, hasta, próximo, vencimiento, estado

**API Rust (Falta):**
- `GET /v1/config/dgii/secuencias` - Lista secuencias desde Tenant.ncfSecuencias o tabla Secuencia
- `POST /v1/config/dgii/secuencias` - Añadir secuencia (simula solicitud OFV)
- `GET /v1/config/empresa`
- `PUT /v1/config/empresa`
- `POST /v1/config/certificado` - Sube P12

**UI Español:**
- `app/(dashboard)/configuracion/page.tsx` - Hub configuración
- `app/(dashboard)/configuracion/empresa/page.tsx` - "Mi Negocio • Datos, Logo, Dirección, Tel"
- `app/(dashboard)/configuracion/dgii/page.tsx` - "DGII • Secuencias e-NCF y Certificado" - Tabla secuencias: Tipo E31/E32, Rango autorizado, Próximo a usar, Vencimiento, Barra progreso uso, botón Solicitar Nueva Secuencia. Upload P12 con password, ambiente selector TesteCF/CerteCF/eCF
- `app/(dashboard)/configuracion/impresora/page.tsx` - "Impresora • Ticket 80mm" - Config IP, test impresión, plantilla ticket con variables
- `app/(dashboard)/configuracion/usuarios/page.tsx` - Ya mencionado

---

#### MÓDULO 12: MÓVIL APP (Expo)

**Objetivo:** Para dueño (ve dashboard), cajero (vende si se cae PC), empleado (solicita adelanto, ve horas).

**Pantallas Español:**
- Login
- Dashboard dueño: Ventas hoy, caja, adelantos pendientes para aprobar
- POS móvil: Productos, carrito, cobrar con QR, offline queue
- Empleado: Mi balance (ganado hoy, disponible adelanto 50%), Solicitar adelanto con motivo, Mis turnos, Mi nómina desprendible
- Gerente: Aprobar adelantos push notification

**API:** Usa mismo gRPC/HTTP core via `@repo/api-client`

---

#### MÓDULO 13: INTEGRACIONES (Futuro pero planear)

- **DGII OFV scrapping** para solicitar secuencias auto
- **Banco Popular API** para pagos nómina dispersión automática
- **WhatsApp** para enviar factura PDF + QR al cliente
- **Impresora** 80mm via Tauri print
- **Balanza** electrónica para pesar plátanos

---

### ROADMAP Módulo por Módulo (Orden Ejecución)

**Fase 0 (Hecho):** Docs, Monorepo, Rust core base, XAdES real, ECF Builder, TigerBeetle mock, POS UI único español, Push a GitHub

**Fase 1 (Semana 1-2): Core Auth + Tenant + Productos + Clientes**
- Módulo 1 Auth, Módulo 2 Productos, Módulo 4 Clientes/Proveedores
- API: POST /v1/auth/*, /v1/productos, /v1/clientes
- UI: Login, Registro negocio, Productos CRUD, Clientes CRUD
- TigerBeetle: Crear cuentas inventario por producto

**Fase 2 (Semana 3-4): Inventario + Compras (para 606)**
- Módulo 3 Inventario, Módulo 6 Compras
- API: /v1/inventario/reservar/comprometer, /v1/compras
- UI: Movimientos Kardex, Compras nueva, Gastos menores
- RFCE ya existe, usar para validar inventario negativo nunca

**Fase 3 (Semana 5-6): POS Completo + Caja**
- Módulo 5 POS, Módulo 9 Caja
- API: /v1/pos/ventas, /v1/caja/abrir/cerrar
- UI: POS con categorías, carrito, pagos, apertura/cierre, historial ventas, impresión 80mm, offline IndexedDB
- Integrar builder real: simplePos -> build_ecf_xml -> sign -> QR

**Fase 4 (Semana 7-8): Contabilidad + Reportes DGII**
- Módulo 7 Contabilidad, Módulo 10 Reportes DGII
- API: /v1/contabilidad/asientos, /v1/reports/606/607 real desde read models
- UI: Libro Mayor, Asientos, Balanza, Reportes DGII export TXT con pre-validación

**Fase 5 (Semana 9-10): Nómina Completa + Adelantos**
- Módulo 8 Nómina (ya 50% hecho)
- API: /v1/empleados, /v1/empleados/:id/clock-in, /v1/nomina/run real con TSS/ISR tabla DGII + TB linked 4 transfers
- UI: Empleados, Turnos, Adelantos tabla completa con aprobar, Run Nómina con cálculo real

**Fase 6 (Semana 11-12): Config DGII + Móvil + Offline Tauri**
- Módulo 11 Config DGII, Módulo 12 Móvil
- API: /v1/config/dgii/secuencias
- UI: Config empresa, DGII secuencias y P12, Móvil Expo full
- Tauri wrapper para POS offline Windows

**Fase 7 (Semana 13-14): Pulido, PSFE Certificación, Multi-tenant SaaS Billing**
- Billing Stripe por negocio, onboarding wizard que guía cliente a certificarse como Emisor Electrónico en OFV
- Solicitud para ser PSFE ante DGII (Proveedor Servicios Facturación Electrónica)

---

### LO QUE FALTA EN API ACTUAL Y PLAN

**Actual existe (Rust core):**
- ✅ POST /v1/ecf/build, /build-sign, /build-sign-send (ECF Builder v1.0 + XAdES + DGII real seed->token->send->poll TrackID)
- ✅ POST /v1/ecf/rfce/build, build-sign, build-sign-send (RFCE <250k)
- ✅ POST /v1/ecf/arecf/build, arecf/build-sign, acecf/build, acecf/build-sign
- ✅ POST /v1/test/sign-demo (self-signed)
- ✅ POST /v1/advances/request, approve (TB pending->posted)
- ✅ GET /v1/employees/:id/balance (mock), POST /v1/payroll/run (mock)
- ✅ GET /v1/reports/606,607 mock
- ✅ TigerBeetle mock client (reserve, post, void) - no real TB yet, logs only
- ✅ EventStore append (hash chain) + NATS notify (parcial)

**Falta y plan:**

| Falta | Plan |
|-------|------|
| Auth Tenant/Usuario | Nuevo `auth_service.rs` + TenantAggregate + read_tenants projector + JWT middleware |
| Productos CRUD | `producto_service.rs` + ProductoAggregate + crear cuenta TB `asset:inventario:{sku}` |
| Clientes/Proveedores CRUD + validación RNC DGII | `cliente_service.rs` + ClienteAggregate + servicio consulta DGII directorio por RNC |
| Inventario reserva/commit real | Ya hay ledger, falta endpoint `/v1/inventario/reservar` que llama `tb_client.reserve` real, no mock |
| Ventas POS con caja | `venta_service.rs` + VentaAggregate + linked transfers TB + evento VentaCompletada + projector read_sales |
| Compras | `compra_service.rs` + CompraAggregate + entrada inventario + costo promedio |
| Contabilidad asientos | `contabilidad_service.rs` + AsientoAggregate, mapea TB accounts a cuenta contable |
| Nómina real TSS/ISR | `nomina_service.rs` real: tabla ISR progresiva 2024, TSS 5.91%, Linked 4 transfers por empleado |
| Caja apertura/cierre | `caja_service.rs` + CajaAggregate + arqueo |
| Config DGII secuencias | `config_service.rs` + SecuenciaAggregate, expiración alerta |
| TigerBeetle real client | Reemplazar mock ledger con `tigerbeetle` crate Rust client real que conecta a :3000 |
| Prisma read models | Definir `apps/web/prisma/schema.prisma` con todas tablas read_* + migrar |
| Móvil Expo full | Ya scaffold, falta pantallas: Login, Dashboard, POS móvil, Adelantos |
| Impresora 80mm | Tauri plugin `tauri-plugin-printer` + plantilla ticket con QR |

---

### ESTRUCTURA CARPETAS FINAL PROPUESTA (Next.js App Router Español)

```
apps/web/app/
  (auth)/
    login/page.tsx - Iniciar Sesión
    registro/page.tsx - Registrar Negocio
  (dashboard)/
    page.tsx - Dashboard Gerencial (ventas hoy, ITBIS, top productos, gráfico)
    pos/
      page.tsx - Terminal Venta (ya existe único diseño)
      caja/page.tsx - Apertura/Cierre/Arqueo
      historial/page.tsx - Ventas del día
    inventario/
      page.tsx - Dashboard inventario valorizado
      productos/page.tsx - Catálogo CRUD
      productos/nuevo/page.tsx - Nuevo producto
      productos/[id]/page.tsx - Detalle + kardex
      categorias/page.tsx - Categorías
      almacenes/page.tsx - Almacenes
      movimientos/page.tsx - Kardex movimientos
      fisico/page.tsx - Inventario físico conteo
    clientes/
      page.tsx - Clientes lista
      nuevo/page.tsx - Nuevo cliente (valida RNC)
      [id]/page.tsx - Detalle balance CxC
    proveedores/
      page.tsx - Proveedores lista
      nuevo/page.tsx - Nuevo proveedor
    compras/
      page.tsx - Compras lista (para 606)
      nueva/page.tsx - Nueva compra con NCF
    gastos/
      page.tsx - Gastos menores B13/E43
    ventas/
      page.tsx - Ventas lista (todas, no solo POS del día)
      [id]/page.tsx - Detalle venta + eNCF + QR + TrackID DGII
      devoluciones/page.tsx - Notas crédito E34
    contabilidad/
      page.tsx - Dashboard contable (balance general, estado resultados)
      libro-mayor/page.tsx - Libro mayor TigerBeetle vivo
      asientos/page.tsx - Asientos
      periodos/page.tsx - Cierre periodo
    nomina/
      page.tsx - Dashboard nómina
      empleados/page.tsx - Equipo
      empleados/nuevo/page.tsx - Nuevo empleado
      empleados/[id]/page.tsx - Detalle horas, adelantos, préstamos
      adelantos/page.tsx - Adelantos EWA 50% tabla aprobar
      turnos/page.tsx - Turnos y asistencia
      prestamos/page.tsx - Préstamos cuotas
      run/page.tsx - Correr nómina quincena
      desprendibles/page.tsx - Desprendibles PDF
    caja/
      page.tsx - Caja actual + movimientos
      bancos/page.tsx - Bancos y conciliación
    reportes/
      page.tsx - Hub reportes
      dgii/
        page.tsx - DGII 606/607/608/609/IT-1
        606/page.tsx - Compras TXT
        607/page.tsx - Ventas + RFCE TXT
        it1/page.tsx - IT-1 borrador
      ventas/page.tsx - Ventas por producto, empleado, hora
      inventario/page.tsx - Valorizado, mermas
      financiero/page.tsx - Estado resultados, balance
    configuracion/
      page.tsx - Hub
      empresa/page.tsx - Mi negocio datos, logo
      dgii/page.tsx - Secuencias e-NCF, Certificado P12, Ambiente
      impresora/page.tsx - Ticket 80mm plantilla
      usuarios/page.tsx - Usuarios y roles
```

Cada `page.tsx` debe ser en español, con el diseño único brutalist/hand-painted que ya iniciamos (bordes 3px black + shadow 6px).

---

### PRÓXIMOS PASOS INMEDIATOS (Esta semana)

1. **Hoy:** Crear estructura carpetas vacías con `page.tsx` placeholder en español "Módulo en construcción" para todos los módulos arriba
2. **Mañana:** Implementar Módulo 1 Auth + Tenant: `auth_service.rs` + JWT + UI login/registro
3. **Pasado:** Módulo 2 Productos: CRUD API + UI + TigerBeetle cuentas inventario
4. **Luego:** Módulo 5 POS completo con caja apertura/cierre + impresión

¿Quieres que empiece scaffoldando todas las carpetas placeholder en español ahora mismo?

