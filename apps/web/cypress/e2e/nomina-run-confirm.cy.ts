import type { TenantSession } from "../support/commands";

describe("Payroll run - confirmation before running", () => {
  let session: TenantSession;
  let periodo: string;

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;
      periodo = `2026-09-confirm-${session.rnc}`;
      cy.corePost(session, "/v1/empleados", { nombre: `E2E Nomina ${session.rnc}`, salario_mensual: "10000.00" });
    });
  });

  it("cancelling the dialog runs nothing; confirming runs the payroll", () => {
    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/nomina/run");

    cy.get("#periodo").type(periodo);
    cy.contains("button", "Correr nómina").click();

    // Submitting only opens the confirmation - it must name the period and
    // warn that the run cannot be undone.
    cy.get('[data-testid="confirm-dialog"]')
      .should("be.visible")
      .and("contain.text", periodo)
      .and("contain.text", "no se puede deshacer");

    cy.get('[data-testid="confirm-dialog-cancel"]').click();
    cy.get('[data-testid="confirm-dialog"]').should("not.exist");

    // Backend truth: cancelling created no payroll period.
    cy.coreGet(session, "/v1/nomina/periodos").then((res) => {
      expect(res.total, "payroll periods after cancelling the dialog").to.eq(0);
    });

    cy.contains("button", "Correr nómina").click();
    cy.get('[data-testid="confirm-dialog-submit"]').click();

    // The run's result table appears with the period label...
    cy.contains("p", periodo, { timeout: 10000 }).should("be.visible");

    // ...and the backend now holds exactly that one period.
    cy.coreGet(session, "/v1/nomina/periodos").then((res) => {
      expect(res.total, "payroll periods after confirming").to.eq(1);
      expect(res.periodos[0].periodo).to.eq(periodo);
    });
  });
});
