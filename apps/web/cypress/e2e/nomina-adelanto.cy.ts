import type { TenantSession } from "../support/commands";
import { parseDOP } from "../support/commands";

describe("Payroll advance (adelanto) - request and approve", () => {
  let session: TenantSession;
  let empleadoNombre: string;

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;
      empleadoNombre = `E2E Adelanto ${session.rnc}`;
      // salario_mensual 10000.00 => 50% cap = 5000.00
      cy.corePost(session, "/v1/empleados", { nombre: empleadoNombre, salario_mensual: "10000.00" });
    });
  });

  it("shows the correct available balance and reconciles the requested/approved amount against the backend", () => {
    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/nomina/adelantos");

    cy.get("#empleado").click().type(empleadoNombre);
    cy.contains("button", empleadoNombre, { timeout: 10000 }).click();

    cy.get('[data-testid="adelanto-disponible"]').should(($el) => {
      expect(parseDOP($el.text())).to.eq("5000.00");
    });

    cy.get("#monto").type("2000.00");
    cy.get('[data-testid="adelanto-solicitar-submit"]').click();

    cy.get('[data-testid="adelanto-row"]', { timeout: 10000 }).should("have.length", 1);
    cy.get('[data-testid="adelanto-row-monto"]').should(($el) => {
      expect(parseDOP($el.text())).to.eq("2000.00");
    });
    cy.get('[data-testid="adelanto-row-estado"]').should("contain.text", "PENDIENTE");

    // Reconcile the just-requested advance against the backend before acting on it.
    cy.coreGet(session, "/v1/nomina/adelantos?pageSize=10").then((page) => {
      expect(page.items, "exactly one adelanto for a fresh tenant").to.have.length(1);
      const adelanto = page.items[0];
      expect(adelanto.monto).to.eq("2000.00");
      expect(adelanto.estado).to.eq("PENDIENTE");
      expect(adelanto.empleado_nombre).to.eq(empleadoNombre);
    });

    cy.get('[data-testid="adelanto-aprobar"]').click();

    cy.get('[data-testid="adelanto-row-estado"]', { timeout: 10000 }).should("contain.text", "APROBADO");

    cy.coreGet(session, "/v1/nomina/adelantos?pageSize=10").then((page) => {
      const adelanto = page.items[0];
      expect(adelanto.estado, "backend estado after approving via the UI").to.eq("APROBADO");
      expect(adelanto.monto, "monto unchanged by approval").to.eq("2000.00");
    });
  });
});
