import type { TenantSession } from "../support/commands";
import { parseDOP } from "../support/commands";

describe("POS sale - money reconciliation", () => {
  let session: TenantSession;
  let sku: string;

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;
      // Open the cash register directly via the core (POS refuses to sell
      // without an open session - see ventas_service::create_venta's
      // CAJA_NO_ABIERTA check) and seed one priced, in-stock product - both
      // via the API, so the spec only drives the UI for the money flow it's
      // actually testing.
      cy.corePost(session, "/v1/caja/abrir", { monto_inicial: "0" });
      sku = `E2E-POS-${session.rnc}`;
      cy.corePost(session, "/v1/productos", {
        sku,
        nombre: "Producto e2e POS",
        itbis_tipo: "GRAVADO_18",
        precio_venta: "100.00",
        stock_actual: "10",
      });
    });
  });

  it("cart total in the DOM matches the backend total after checkout", () => {
    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/pos");

    cy.get(`[data-testid="pos-product-card"][data-sku="${sku}"]`).click();

    // precio_venta 100.00 x1, GRAVADO_18 => subtotal 100.00, itbis 18.00, total 118.00
    cy.get('[data-testid="pos-cart-total"]').should(($el) => {
      expect(parseDOP($el.text())).to.eq("118.00");
    });

    cy.get('[data-testid="pos-cobrar-submit"]').click();

    cy.get('[data-testid="pos-venta-receipt"]', { timeout: 10000 })
      .should("have.attr", "data-venta-id")
      .and("not.be.empty");

    cy.get('[data-testid="pos-venta-receipt"]')
      .invoke("attr", "data-venta-id")
      .then((ventaId) => {
        expect(ventaId, "venta id on the receipt").to.be.a("string");

        // The number on screen, independently...
        cy.get('[data-testid="pos-venta-total"]')
          .invoke("text")
          .then(parseDOP)
          .then((domTotal) => {
            // ...reconciled against backend truth fetched directly from the
            // core (bypassing the Next.js BFF this UI flow just used).
            cy.coreGet(session, `/v1/ventas/${ventaId}`).then((venta) => {
              expect(domTotal, "DOM total vs backend venta.total").to.eq(venta.total);
              expect(venta.total).to.eq("118.00");
              expect(venta.subtotal).to.eq("100.00");
              expect(venta.itbis_total).to.eq("18.00");
            });
          });
      });
  });
});
