import type { TenantSession } from "../support/commands";
import { parseDOP } from "../support/commands";

describe("POS - cash received and change due (display only)", () => {
  let session: TenantSession;
  let sku: string;

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;
      cy.corePost(session, "/v1/caja/abrir", { monto_inicial: "0" });
      sku = `E2E-CAMBIO-${session.rnc}`;
      cy.corePost(session, "/v1/productos", {
        sku,
        nombre: "Producto e2e cambio",
        itbis_tipo: "GRAVADO_18",
        precio_venta: "100.00",
        stock_actual: "10",
      });
    });
  });

  it("computes change / shortfall, hides for non-cash, and never blocks or alters the sale", () => {
    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/pos");

    cy.get(`[data-testid="pos-product-card"][data-sku="${sku}"]`).click();
    // precio 100.00 x1, GRAVADO_18 => total 118.00
    cy.get('[data-testid="pos-cart-total"]').should(($el) => {
      expect(parseDOP($el.text())).to.eq("118.00");
    });

    // Cash is the default method, so the optional field is already there.
    cy.get('[data-testid="pos-efectivo-recibido"]').type("200");
    cy.get('[data-testid="pos-cambio"]').should(($el) => {
      expect($el.text()).to.not.contain("Faltan");
      expect(parseDOP($el.text())).to.eq("82.00");
    });

    // Short of the total: warns with the exact shortfall.
    cy.get('[data-testid="pos-efectivo-recibido"]').clear().type("100");
    cy.get('[data-testid="pos-cambio"]').should(($el) => {
      expect($el.text()).to.contain("Faltan");
      expect(parseDOP($el.text())).to.eq("18.00");
    });

    // Exact cash: zero change, not a shortfall.
    cy.get('[data-testid="pos-efectivo-recibido"]').clear().type("118");
    cy.get('[data-testid="pos-cambio"]').should(($el) => {
      expect($el.text()).to.not.contain("Faltan");
      expect(parseDOP($el.text())).to.eq("0.00");
    });

    // Switching to a non-cash method removes the field entirely.
    cy.get("#metodo").select("TARJETA");
    cy.get('[data-testid="pos-efectivo-recibido"]').should("not.exist");
    cy.get('[data-testid="pos-cambio"]').should("not.exist");

    // Back to cash: the previous amount was reset, not silently kept.
    cy.get("#metodo").select("EFECTIVO");
    cy.get('[data-testid="pos-efectivo-recibido"]').should("have.value", "");

    // A shortfall never blocks "Cobrar" - the amount is display-only.
    cy.get('[data-testid="pos-efectivo-recibido"]').type("50");
    cy.get('[data-testid="pos-cobrar-submit"]').should("not.be.disabled").click();

    cy.get('[data-testid="pos-venta-receipt"]', { timeout: 10000 })
      .should("have.attr", "data-venta-id")
      .then((ventaId) => {
        // Backend truth: the sale is for the cart total, unaffected by the
        // amount typed into the cash field.
        cy.coreGet(session, `/v1/ventas/${ventaId}`).then((venta) => {
          expect(venta.total).to.eq("118.00");
        });
      });

    // A new sale starts with an empty cash field.
    cy.contains("button", "Nueva venta").click();
    cy.get(`[data-testid="pos-product-card"][data-sku="${sku}"]`).click();
    cy.get('[data-testid="pos-efectivo-recibido"]').should("have.value", "");
  });
});
