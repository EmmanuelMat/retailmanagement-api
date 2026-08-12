import type { TenantSession } from "../support/commands";
import { normalizeDecimal, parseDOP } from "../support/commands";

describe("Libro mayor - every row in the DOM matches backend truth, and the ledger balances", () => {
  let session: TenantSession;

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;

      // A mix of money-writing flows, all seeded directly via the API (each
      // flow's own UI is covered by its dedicated spec) - this spec is only
      // about the libro mayor page's numbers being trustworthy.
      cy.corePost(session, "/v1/caja/abrir", { monto_inicial: "1000.00" });
      cy.corePost(session, "/v1/productos", {
        sku: `E2E-LEDGER-V-${session.rnc}`,
        nombre: "Producto e2e ledger venta",
        itbis_tipo: "GRAVADO_18",
        precio_venta: "75.00",
        stock_actual: "20",
      }).then((producto) => {
        cy.corePost(session, "/v1/ventas", { items: [{ producto_id: producto.id, cantidad: "3" }], metodo_pago: "EFECTIVO" });
      });
      cy.corePost(session, "/v1/productos", {
        sku: `E2E-LEDGER-C-${session.rnc}`,
        nombre: "Producto e2e ledger compra",
        itbis_tipo: "GRAVADO_16",
        precio_venta: "0",
        stock_actual: "0",
      }).then((producto) => {
        cy.corePost(session, "/v1/compras", {
          items: [{ producto_id: producto.id, cantidad: "10", costo_unitario: "22.50", itbis_tipo: "GRAVADO_16" }],
          metodo_pago: "EFECTIVO",
        });
      });
      cy.corePost(session, "/v1/gastos", { concepto: "Servicios e2e", categoria: "SERVICIOS", monto: "1250.75" });
      cy.corePost(session, "/v1/empleados", { nombre: "Empleado e2e ledger", salario_mensual: "20000.00" }).then((empleado) => {
        cy.corePost(session, "/v1/nomina/adelantos", { empleado_id: empleado.id, monto: "3000.00" }).then((adelanto) => {
          cy.corePost(session, `/v1/nomina/adelantos/${adelanto.id}/aprobar`, {});
        });
        cy.corePost(session, "/v1/nomina/run", { periodo: `2026-08-ledger-cy-${session.rnc}` });
      });
      cy.corePost(session, "/v1/contabilidad/sincronizar", {});
    });
  });

  it("every rendered account row matches the backend, and total debe equals total haber", () => {
    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/contabilidad/libro-mayor");

    cy.get('[data-testid="libro-mayor-row"]', { timeout: 10000 }).should("have.length.greaterThan", 0);

    cy.coreGet(session, "/v1/contabilidad/libro-mayor?pageSize=200").then((page) => {
      const backendByAccount = new Map<string, { debe: string; haber: string; saldo: string }>(
        page.items.map((row: any) => [row.cuenta, row])
      );

      let domTotalDebe = 0;
      let domTotalHaber = 0;

      cy.get('[data-testid="libro-mayor-row"]').each(($row) => {
        const cuenta = $row.attr("data-cuenta")!;
        const backendRow = backendByAccount.get(cuenta);
        expect(backendRow, `backend should have a row for account ${cuenta}`).to.exist;

        const domDebe = parseDOP($row.find('[data-testid="libro-mayor-debe"]').text());
        const domHaber = parseDOP($row.find('[data-testid="libro-mayor-haber"]').text());
        const domSaldo = parseDOP($row.find('[data-testid="libro-mayor-saldo"]').text());

        expect(domDebe, `${cuenta} debe`).to.eq(normalizeDecimal(backendRow!.debe));
        expect(domHaber, `${cuenta} haber`).to.eq(normalizeDecimal(backendRow!.haber));
        expect(domSaldo, `${cuenta} saldo`).to.eq(normalizeDecimal(backendRow!.saldo));

        domTotalDebe += Number(domDebe);
        domTotalHaber += Number(domHaber);
      }).then(() => {
        // The golden invariant, verified straight off what's rendered on screen.
        expect(domTotalDebe.toFixed(2), "SUM(debe) across every rendered row").to.eq(domTotalHaber.toFixed(2));

        const backendTotalDebe = page.items.reduce((sum: number, r: any) => sum + Number(r.debe), 0);
        const backendTotalHaber = page.items.reduce((sum: number, r: any) => sum + Number(r.haber), 0);
        expect(domTotalDebe.toFixed(2), "DOM total debe vs backend total debe").to.eq(backendTotalDebe.toFixed(2));
        expect(domTotalHaber.toFixed(2), "DOM total haber vs backend total haber").to.eq(backendTotalHaber.toFixed(2));
      });
    });
  });
});
