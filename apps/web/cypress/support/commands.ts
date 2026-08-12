// Custom commands for the money e2e suite. See scripts/e2e/run-local.sh /
// .github/workflows/e2e.yml for how the Next.js app and the Rust core it
// proxies to are both already running before these specs execute.

export interface TenantSession {
  token: string;
  rnc: string;
  usuario: Record<string, unknown>;
  tenant: Record<string, unknown>;
}

/** A fresh, valid (9-digit numeric) RNC per call - mirrors
 * services/core/tests/common/mod.rs::unique_rnc so backend and UI specs
 * can't collide on a tenant even if run in the same CI job. */
function uniqueRnc(): string {
  const n = Math.floor(Math.random() * 1_000_000_00).toString().padStart(8, "0");
  return `9${n}`;
}

Cypress.Commands.add("registerTenant", () => {
  const rnc = uniqueRnc();
  return cy
    .request({
      method: "POST",
      url: `${Cypress.env("CORE_URL")}/v1/auth/register`,
      body: {
        rnc,
        razon_social: `Test Tenant ${rnc}`,
        direccion: "Calle Test #1",
        admin_nombre: "Test Admin",
        admin_email: `admin-${rnc}@e2e-test.local`,
        admin_password: "TestPassword123!",
        // DGII e-CF signing/sending is explicitly out of scope for this
        // money-correctness suite (external sandbox dependency, compliance
        // concern not a money one - see plan). Disabling it here stops
        // apps/web/app/(dashboard)/pos/page.tsx from firing its automatic
        // post-sale emitir-ecf call, which would otherwise fail against a
        // tenant with no DGII certificate configured.
        factura_electronica_activa: false,
      },
    })
    .then((res) => {
      expect(res.status, `register tenant ${rnc}`).to.eq(200);
      return {
        token: res.body.token,
        rnc,
        usuario: res.body.usuario,
        tenant: res.body.tenant,
      } as TenantSession;
    });
});

/**
 * Programmatic login: sets both the `token` cookie (read by
 * apps/web/middleware.ts to gate page navigation) and the localStorage
 * keys apps/web/lib/api.ts + the login page manage (`token`, `usuario`,
 * `tenant` - see apps/web/app/(auth)/login/page.tsx) - so specs don't have
 * to drive the login form to exercise a money flow.
 *
 * localStorage is origin-scoped and only reachable via `cy.window()` once
 * some page on that origin has actually loaded - call `cy.visit("/login")`
 * (any page works; `/login` is in middleware's public allow-list) *before*
 * `cy.loginAs(...)`, then `cy.visit(<target page>)` after. The cookie half
 * doesn't have this restriction, but keep both after the same initial visit
 * for one obvious call order across every spec.
 */
Cypress.Commands.add("loginAs", (session: TenantSession) => {
  cy.setCookie("token", session.token, { path: "/" });
  // A `.then()` callback that returns undefined makes Cypress carry the
  // previous subject (the window) forward unchanged - not `undefined` - so
  // this intentionally yields the window, not void; see the `Chainable<any>`
  // return type in the ambient `loginAs` declaration below.
  return cy.window().then((win) => {
    win.localStorage.setItem("token", session.token);
    win.localStorage.setItem("usuario", JSON.stringify(session.usuario));
    win.localStorage.setItem("tenant", JSON.stringify(session.tenant));
  });
});

/** Backend truth: GET a core endpoint directly (not through the Next BFF)
 * with the session's token, for reconciling against what the DOM shows. */
Cypress.Commands.add("coreGet", (session: TenantSession, path: string) => {
  return cy
    .request({
      method: "GET",
      url: `${Cypress.env("CORE_URL")}${path}`,
      headers: { Authorization: `Bearer ${session.token}` },
    })
    .then((res) => res.body);
});

/** Backend truth/setup: POST a core endpoint directly, for seeding fixture
 * data (products, employees) without exercising the UI under test. */
Cypress.Commands.add("corePost", (session: TenantSession, path: string, body: any) => {
  return cy
    .request({
      method: "POST",
      url: `${Cypress.env("CORE_URL")}${path}`,
      headers: { Authorization: `Bearer ${session.token}` },
      body,
    })
    .then((res) => res.body);
});

/** Parses a formatDOP-rendered string (e.g. "RD$1,234.50", see
 * packages/ui/src/utils.ts) back into a canonical decimal string - never
 * compare formatted display text directly, and never round-trip through a
 * float for money. */
export function parseDOP(text: string): string {
  const cleaned = text.replace(/[^0-9.-]/g, "");
  if (!cleaned) throw new Error(`could not parse a DOP amount out of ${JSON.stringify(text)}`);
  // Normalize "-0.00" etc and guarantee 2 decimals for a stable string compare.
  return Number(cleaned).toFixed(2);
}

/**
 * Normalizes a raw backend Decimal string (e.g. from `cy.coreGet`) to the
 * same canonical 2-decimal form `parseDOP` produces from the DOM. Backend
 * Decimal strings are usually already "123.45", but `COALESCE(SUM(x), 0)`
 * over zero matching rows comes back scale-less as `"0"`, not `"0.00"` -
 * comparing that raw against a DOM-parsed `"0.00"` fails on formatting, not
 * on the actual number. Always normalize both sides before comparing;
 * never compare a raw backend string straight against DOM text.
 */
export function normalizeDecimal(raw: string): string {
  return Number(raw).toFixed(2);
}

declare global {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace Cypress {
    interface Chainable {
      registerTenant(): Chainable<TenantSession>;
      loginAs(session: TenantSession): Chainable<any>;
      coreGet(session: TenantSession, path: string): Chainable<any>;
      corePost(session: TenantSession, path: string, body: any): Chainable<any>;
    }
  }
}
