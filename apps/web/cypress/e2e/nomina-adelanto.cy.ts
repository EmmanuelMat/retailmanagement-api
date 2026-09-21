import type { TenantSession } from "../support/commands";
import { normalizeDecimal, parseDOP } from "../support/commands";

describe("Payroll advance (adelanto) - request and approve", () => {
  let session: TenantSession;
  let empleadoNombre: string;
  let empleadoId: string;

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;
      empleadoNombre = `E2E Adelanto ${session.rnc}`;
      // salario_mensual 10000.00 => 50% cap = 5000.00
      cy.corePost(session, "/v1/empleados", { nombre: empleadoNombre, salario_mensual: "10000.00" }).then((emp) => {
        empleadoId = emp.id;
      });
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

    // Approving moves cash out of caja immediately, so the click only opens a
    // confirmation - the approval happens on the dialog's confirm button.
    cy.get('[data-testid="adelanto-aprobar"]').click();
    cy.get('[data-testid="confirm-dialog"]').should("be.visible");
    cy.get('[data-testid="confirm-dialog-submit"]').click();

    cy.get('[data-testid="adelanto-row-estado"]', { timeout: 10000 }).should("contain.text", "APROBADO");

    cy.coreGet(session, "/v1/nomina/adelantos?pageSize=10").then((page) => {
      const adelanto = page.items[0];
      expect(adelanto.estado, "backend estado after approving via the UI").to.eq("APROBADO");
      expect(adelanto.monto, "monto unchanged by approval").to.eq("2000.00");
    });
  });

  it("asks for confirmation before approving, and cancelling the dialog changes nothing", () => {
    cy.corePost(session, "/v1/caja/abrir", { monto_inicial: "0" });
    cy.corePost(session, "/v1/nomina/adelantos", { empleado_id: empleadoId, monto: "1500.00" });

    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/nomina/adelantos");

    cy.get('[data-testid="adelanto-row-estado"]', { timeout: 10000 }).should("contain.text", "PENDIENTE");
    cy.get('[data-testid="adelanto-aprobar"]').click();

    // The dialog names who and how much, and says the money leaves caja now.
    cy.get('[data-testid="confirm-dialog"]')
      .should("be.visible")
      .and("contain.text", empleadoNombre)
      .and("contain.text", "caja")
      .invoke("text")
      .should("match", /1,500\.00/);

    cy.get('[data-testid="confirm-dialog-cancel"]').click();
    cy.get('[data-testid="confirm-dialog"]').should("not.exist");
    cy.get('[data-testid="adelanto-row-estado"]').should("contain.text", "PENDIENTE");

    // Backend truth: nothing was approved and no cash left the caja.
    cy.coreGet(session, "/v1/nomina/adelantos?pageSize=10").then((page) => {
      expect(page.items, "still exactly one adelanto").to.have.length(1);
      expect(page.items[0].estado, "backend estado after cancelling the dialog").to.eq("PENDIENTE");
    });
    cy.coreGet(session, "/v1/caja/resumen").then((resumen) => {
      expect(normalizeDecimal(resumen.egresos), "caja egresos after cancelling the dialog").to.eq("0.00");
    });
  });

  it("asks for confirmation before rejecting, then rejects on confirm", () => {
    cy.corePost(session, "/v1/nomina/adelantos", { empleado_id: empleadoId, monto: "800.00" });

    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/nomina/adelantos");

    cy.get('[data-testid="adelanto-row-estado"]', { timeout: 10000 }).should("contain.text", "PENDIENTE");
    cy.get('[data-testid="adelanto-rechazar"]').click();

    cy.get('[data-testid="confirm-dialog"]').should("be.visible").and("contain.text", empleadoNombre);
    // Nothing happens until the dialog is confirmed.
    cy.get('[data-testid="adelanto-row-estado"]').should("contain.text", "PENDIENTE");

    cy.get('[data-testid="confirm-dialog-submit"]').click();
    cy.get('[data-testid="adelanto-row-estado"]', { timeout: 10000 }).should("contain.text", "RECHAZADO");

    cy.coreGet(session, "/v1/nomina/adelantos?pageSize=10").then((page) => {
      expect(page.items[0].estado, "backend estado after rejecting via the UI").to.eq("RECHAZADO");
    });
  });
});
