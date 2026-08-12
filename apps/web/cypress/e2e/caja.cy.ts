import type { TenantSession } from "../support/commands";
import { parseDOP } from "../support/commands";

describe("Caja (cash register) - open, a cash sale, close", () => {
  let session: TenantSession;

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;
    });
  });

  it("resumen and cierre numbers in the DOM match backend truth", () => {
    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/caja");

    // Drive the actual "abrir caja" form - this is the UI flow under test.
    cy.get("#inicial").type("500.00");
    cy.get('[data-testid="caja-abrir-submit"]').click();

    cy.get('[data-testid="caja-monto-inicial"]', { timeout: 10000 }).should(($el) => {
      expect(parseDOP($el.text())).to.eq("500.00");
    });

    // Seed a cash sale via the API (POS's own UI flow is covered by
    // pos-sale.cy.ts) so there's a real ingreso to reconcile against.
    cy.corePost(session, "/v1/productos", {
      sku: `E2E-CAJA-${session.rnc}`,
      nombre: "Producto e2e caja",
      itbis_tipo: "GRAVADO_18",
      precio_venta: "200.00",
      stock_actual: "5",
    }).then((producto) => {
      cy.corePost(session, "/v1/ventas", {
        items: [{ producto_id: producto.id, cantidad: "1" }],
        metodo_pago: "EFECTIVO",
      }).then((venta) => {
        expect(venta.total).to.eq("236.00"); // 200.00 + 18% ITBIS

        // Re-visit to force the page to refetch /api/caja/resumen and pick
        // up the movement the sale above just created.
        cy.visit("/caja");

        cy.get('[data-testid="caja-ingresos"]', { timeout: 10000 }).should(($el) => {
          expect(parseDOP($el.text())).to.eq("236.00");
        });
        cy.get('[data-testid="caja-saldo-esperado"]').should(($el) => {
          expect(parseDOP($el.text())).to.eq("736.00"); // 500.00 + 236.00
        });

        cy.coreGet(session, "/v1/caja/resumen").then((resumen) => {
          cy.get('[data-testid="caja-saldo-esperado"]')
            .invoke("text")
            .then(parseDOP)
            .then((domSaldo) => {
              expect(domSaldo, "DOM saldo esperado vs backend saldo_actual").to.eq(resumen.saldo_actual);
            });

          // Close the register RD$10 short via the actual "cerrar caja" form.
          const esperado = Number(resumen.saldo_actual);
          const contado = (esperado - 10).toFixed(2);
          cy.get("#final").type(contado);
          cy.get('[data-testid="caja-cerrar-submit"]').click();

          cy.get('[data-testid="caja-cierre-diferencia"]', { timeout: 10000 }).should(($el) => {
            expect(parseDOP($el.text())).to.eq("-10.00");
          });
          cy.get('[data-testid="caja-cierre-esperado"]').should(($el) => {
            expect(parseDOP($el.text())).to.eq(esperado.toFixed(2));
          });
          cy.get('[data-testid="caja-cierre-contado"]').should(($el) => {
            expect(parseDOP($el.text())).to.eq(contado);
          });
        });
      });
    });
  });
});
