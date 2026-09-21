import type { TenantSession } from "../support/commands";
import { parseDOP } from "../support/commands";

// Devolución parcial desde el detalle de la venta: el cajero elige cuánto
// devuelve de cada línea, confirma en el ConfirmDialog, y lo que la UI muestra
// se reconcilia contra la Nota de Crédito que el core realmente grabó.
describe("Devolución parcial de una venta", () => {
  let session: TenantSession;
  let ventaId: string;

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;
      // Sin factura electrónica activa la devolución se registra sin e-CF: el
      // pipeline de firma DGII necesita un P12 real y está fuera de esta suite.
      // (PUT en línea: el harness compartido solo expone coreGet/corePost.)
      cy.request({
        method: "PUT",
        url: `${Cypress.env("CORE_URL")}/v1/config/empresa`,
        headers: { Authorization: `Bearer ${session.token}` },
        body: {
          razon_social: `Test Tenant ${session.rnc}`,
          direccion: "Calle Test #1",
          ambiente_dgii: "TesteCF",
          factura_electronica_activa: false,
        },
      });
      cy.corePost(session, "/v1/caja/abrir", { monto_inicial: "0" });
      cy.corePost(session, "/v1/productos", {
        sku: `E2E-DEV-${session.rnc}`,
        nombre: "Refresco e2e",
        itbis_tipo: "GRAVADO_18",
        precio_venta: "100.00",
        costo: "60.00",
        stock_actual: "20",
      }).then((producto) => {
        // 4 x 100.00 => subtotal 400.00, ITBIS 72.00, total 472.00.
        cy.corePost(session, "/v1/ventas", {
          items: [{ producto_id: producto.id, cantidad: "4" }],
          metodo_pago: "EFECTIVO",
        }).then((venta) => {
          ventaId = venta.id;
        });
      });
    });
  });

  it("devuelve 1 de 4 unidades y la venta sigue COMPLETADA", () => {
    cy.visit("/login");
    cy.loginAs(session);
    cy.visit(`/ventas/${ventaId}`);

    cy.get('[data-testid="abrir-devolucion"]').click();
    cy.get('[data-testid="devolucion-dialog"]').should("be.visible");

    // Un cuarto de la venta: 472.00 / 4 = 118.00.
    cy.get('[data-testid^="devolver-cantidad-"]').first().clear().type("1");
    cy.get('[data-testid="devolucion-motivo"]').type("Producto defectuoso");
    cy.get('[data-testid="devolucion-total"]').should(($el) => {
      expect(parseDOP($el.text())).to.eq("118.00");
    });

    // Paso de confirmación obligatorio: mueve dinero e inventario.
    cy.get('[data-testid="devolucion-continuar"]').click();
    cy.get('[data-testid="confirm-dialog"]').should("be.visible");
    cy.get('[data-testid="confirm-dialog-submit"]').click();

    // Verdad del backend, no del DOM: la nota acredita 118.00, es parcial, y
    // la venta no quedó anulada por devolver una unidad.
    cy.coreGet(session, `/v1/ventas/${ventaId}/devoluciones`).then((d) => {
      expect(d.notas, "una nota de crédito").to.have.length(1);
      expect(d.notas[0].total, "total acreditado").to.eq("118.00");
      expect(d.notas[0].es_parcial, "es parcial").to.eq(true);
      expect(d.lineas[0].cantidad_devuelta, "cantidad devuelta de la línea").to.eq("1.00");
    });
    cy.coreGet(session, `/v1/ventas/${ventaId}`).then((venta) => {
      expect(venta.estado, "la venta sigue viva tras una devolución parcial").to.eq("COMPLETADA");
    });

    // Y la devolución aparece en el listado de Devoluciones.
    cy.visit("/ventas/devoluciones");
    cy.get('[data-testid="devolucion-row"]').should("have.length", 1).and("contain.text", "Parcial");
  });
});
