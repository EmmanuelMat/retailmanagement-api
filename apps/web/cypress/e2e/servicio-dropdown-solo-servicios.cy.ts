import type { TenantSession } from "../support/commands";

/**
 * El selector de renglones de una orden de servicio o cotización lista solo
 * servicios, sea cual sea el tipo de negocio (registerTenant crea un tenant
 * COLMADO, el default). Los productos se agregan en la pestaña Materiales
 * de la orden, que sí debe seguir listándolos.
 */
describe("Órdenes de servicio y cotizaciones - el selector de renglones lista solo servicios", () => {
  let session: TenantSession;
  let servicioId: string;
  const servicio = "Instalación de aire acondicionado";
  const producto = "Tubo de cobre 3/8";

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;
      cy.corePost(session, "/v1/productos", {
        sku: `SRV-${s.rnc}`,
        nombre: servicio,
        itbis_tipo: "GRAVADO_18",
        tipo: "SERVICIO",
      }).then((p) => {
        servicioId = p.id;
      });
      cy.corePost(session, "/v1/productos", {
        sku: `PRD-${s.rnc}`,
        nombre: producto,
        itbis_tipo: "GRAVADO_18",
        tipo: "PRODUCTO",
        precio_venta: "250",
      });
    });
  });

  function selectorListaSoloServicio(select: () => Cypress.Chainable) {
    select().find("option").should("contain.text", servicio);
    select().find("option").should("not.contain.text", producto);
  }

  it("nueva orden de servicio", () => {
    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/ordenes-servicio/nueva");
    selectorListaSoloServicio(() => cy.contains("option", "Producto o servicio…").parent());
  });

  it("detalle de orden: Items solo servicios, Materiales sigue con productos", () => {
    cy.corePost(session, "/v1/ordenes-servicio", {
      items: [{ producto_id: servicioId, cantidad: "1", precio_unitario: "1500" }],
    }).then((orden: any) => {
      cy.visit("/login");
      cy.loginAs(session);
      cy.visit(`/ordenes-servicio/${orden.id}`);

      cy.contains("button", "Items").click();
      // Un select por renglón del formulario de agregar; es el que lista el catálogo.
      selectorListaSoloServicio(() => cy.contains("option", servicio).parent());

      cy.contains("button", "Materiales").click();
      cy.contains("option", producto).should("exist");
    });
  });

  it("nueva cotización", () => {
    cy.visit("/login");
    cy.loginAs(session);
    cy.visit("/cotizaciones/nueva");
    selectorListaSoloServicio(() => cy.contains("option", "Producto…").parent());
  });

  it("detalle de cotización", () => {
    cy.corePost(session, "/v1/cotizaciones", {
      items: [{ producto_id: servicioId, cantidad: "1", precio_unitario: "1500" }],
    }).then((cot: any) => {
      cy.visit("/login");
      cy.loginAs(session);
      cy.visit(`/cotizaciones/${cot.id}`);
      selectorListaSoloServicio(() => cy.contains("option", "Producto…").parent());
    });
  });
});
