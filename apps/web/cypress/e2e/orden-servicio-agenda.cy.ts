import type { TenantSession } from "../support/commands";

/**
 * Fase E - programación de técnicos. Cubre las tres cosas que la pantalla
 * tiene que hacer bien: mostrar el conflicto de agenda en vez de un error
 * crudo, dejar asignar igual ("Asignar de todos modos", regla blanda), y
 * listar el trabajo del día en la agenda agrupado por técnico.
 */
describe("Órdenes de servicio - agenda y conflictos de técnico", () => {
  let session: TenantSession;
  let servicioId: string;
  let tecnicoId: string;
  const dia = "2026-11-10";

  beforeEach(() => {
    cy.registerTenant().then((s) => {
      session = s;
      cy.corePost(session, "/v1/productos", {
        sku: `SRV-${s.rnc}`,
        nombre: "Mantenimiento de aire acondicionado",
        itbis_tipo: "GRAVADO_18",
        tipo: "SERVICIO",
      }).then((p) => {
        servicioId = p.id;
      });
      cy.corePost(session, "/v1/empleados", { nombre: "Juan Técnico", salario_mensual: "30000" }).then((e) => {
        tecnicoId = e.id;
      });
    });
  });

  /** Una orden programada con bloque horario, opcionalmente ya con técnico. */
  function crearOrden(horaInicio: string, horaFin: string, conTecnico: boolean) {
    return cy
      .corePost(session, "/v1/ordenes-servicio", {
        fecha_programada: dia,
        hora_inicio: horaInicio,
        hora_fin: horaFin,
        items: [{ producto_id: servicioId, cantidad: "1", precio_unitario: "1500" }],
      })
      .then((orden) => {
        if (conTecnico) {
          cy.corePost(session, `/v1/ordenes-servicio/${orden.id}/tecnicos`, { empleado_id: tecnicoId, rol: "TECNICO_PRINCIPAL" });
        }
        return cy.wrap(orden);
      });
  }

  it("muestra el conflicto al asignar un técnico ya ocupado y permite asignarlo de todos modos", () => {
    // 09:00-11:00 ya ocupada por el técnico.
    crearOrden("09:00", "11:00", true).then((ocupada: any) => {
      const codigoOcupada = `OS-${ocupada.id.slice(0, 8).toUpperCase()}`;

      // 10:00-12:00 pisa el bloque anterior.
      crearOrden("10:00", "12:00", false).then((nueva: any) => {
        cy.visit("/login");
        cy.loginAs(session);
        cy.visit(`/ordenes-servicio/${nueva.id}`);

        cy.contains("button", "Técnicos").click();
        cy.get("select").first().select("Juan Técnico");
        cy.get('[data-testid="asignar-tecnico"]').click();

        // El 409 se presenta como una decisión, no como un error: el diálogo
        // nombra la orden que choca, con su horario.
        cy.get('[data-testid="confirm-dialog"]')
          .should("be.visible")
          .and("contain.text", "Conflicto de agenda")
          .and("contain.text", codigoOcupada)
          .and("contain.text", "09:00 – 11:00");

        // Cancelar no asigna nada.
        cy.get('[data-testid="confirm-dialog-cancel"]').click();
        cy.get('[data-testid="confirm-dialog"]').should("not.exist");
        cy.coreGet(session, `/v1/ordenes-servicio/${nueva.id}`).then((res) => {
          expect(res.tecnicos, "técnicos tras cancelar el diálogo").to.have.length(0);
        });

        // Confirmar sí, y el backend lo registra.
        cy.get('[data-testid="asignar-tecnico"]').click();
        cy.get('[data-testid="confirm-dialog-submit"]').click();
        cy.get('[data-testid="confirm-dialog"]').should("not.exist");

        cy.coreGet(session, `/v1/ordenes-servicio/${nueva.id}`).then((res) => {
          expect(res.tecnicos, "técnicos tras confirmar el conflicto").to.have.length(1);
          expect(res.tecnicos[0].empleado_id).to.eq(tecnicoId);
        });

        // Y el override queda auditado (regla blanda, nunca silenciosa).
        cy.coreGet(session, `/v1/auditoria?entidad=orden_servicio&entidadId=${nueva.id}&pageSize=50`).then((res) => {
          const acciones = res.items.map((e: any) => e.accion);
          expect(acciones, "acciones auditadas").to.include("ORDEN_SERVICIO_CONFLICTO_AGENDA_OMITIDO");
        });
      });
    });
  });

  it("no molesta cuando los bloques son adyacentes", () => {
    crearOrden("09:00", "11:00", true).then(() => {
      // Empieza justo cuando la otra termina: no es doble reserva.
      crearOrden("11:00", "13:00", false).then((nueva: any) => {
        cy.visit("/login");
        cy.loginAs(session);
        cy.visit(`/ordenes-servicio/${nueva.id}`);

        cy.contains("button", "Técnicos").click();
        cy.get("select").first().select("Juan Técnico");
        cy.get('[data-testid="asignar-tecnico"]').click();

        cy.get('[data-testid="confirm-dialog"]').should("not.exist");
        cy.contains("Juan Técnico").should("be.visible");
        cy.coreGet(session, `/v1/ordenes-servicio/${nueva.id}`).then((res) => {
          expect(res.tecnicos, "asignación directa sin diálogo").to.have.length(1);
        });
      });
    });
  });

  it("la agenda lista el trabajo del día agrupado por técnico", () => {
    crearOrden("09:00", "11:00", true).then((orden: any) => {
      const codigo = `OS-${orden.id.slice(0, 8).toUpperCase()}`;

      cy.visit("/login");
      cy.loginAs(session);

      // Se llega desde el listado de órdenes.
      cy.visit("/ordenes-servicio");
      cy.get('[data-testid="ver-agenda"]').click();
      cy.location("pathname").should("eq", "/ordenes-servicio/agenda");

      // La agenda abre en "hoy"; esta orden es de una fecha fija.
      cy.get("#agenda-dia").clear().type(dia);

      cy.get('[data-testid="agenda-grupo"]').should("have.length", 1).within(() => {
        cy.contains("Juan Técnico").should("be.visible");
        cy.get('[data-testid="agenda-orden"]')
          .should("have.length", 1)
          .and("contain.text", codigo)
          .and("contain.text", "09:00 – 11:00");
      });

      // Filtrar por otro técnico la vacía.
      cy.corePost(session, "/v1/empleados", { nombre: "Otro Técnico", salario_mensual: "30000" }).then(() => {
        cy.reload();
        cy.get("#agenda-dia").clear().type(dia);
        cy.get("#agenda-empleado").select("Otro Técnico");
        cy.get('[data-testid="agenda-vacia"]').should("be.visible");
      });
    });
  });

  it("guarda el bloque horario desde la pestaña Programación", () => {
    cy.corePost(session, "/v1/ordenes-servicio", {
      items: [{ producto_id: servicioId, cantidad: "1", precio_unitario: "1500" }],
    }).then((orden: any) => {
      cy.visit("/login");
      cy.loginAs(session);
      cy.visit(`/ordenes-servicio/${orden.id}`);

      cy.contains("button", "Programación").click();
      cy.get("#prog-fecha").type(dia);
      cy.get("#prog-desde").type("14:00");
      cy.get("#prog-hasta").type("16:00");
      cy.get('[data-testid="guardar-programacion"]').click();

      cy.get('[data-testid="orden-programada"]').should("contain.text", "14:00 – 16:00");
      cy.coreGet(session, `/v1/ordenes-servicio/${orden.id}`).then((res) => {
        expect(res.fecha_programada, "fecha programada en el backend").to.eq(dia);
        expect(res.hora_inicio).to.eq("14:00:00");
        expect(res.hora_fin).to.eq("16:00:00");
        expect(res.estado, "fijar la fecha agenda la orden").to.eq("PROGRAMADA");
      });
    });
  });
});
