import { defineConfig } from "cypress";

export default defineConfig({
  e2e: {
    baseUrl: "http://localhost:3000",
    // Wider than Cypress's 1000px default: the POS page's cart panel only
    // sits beside the product grid at Tailwind's `lg:` breakpoint (1024px);
    // below that it stacks as a single column and, combined with the cart
    // panel's `sticky` positioning, can overlap the product grid instead of
    // flowing beneath it - a real (if awkward) responsive layout the POS
    // app was never designed to be driven at, and this suite drives it as
    // the desktop point-of-sale terminal it's actually used as.
    viewportWidth: 1440,
    viewportHeight: 900,
    specPattern: "cypress/e2e/**/*.cy.ts",
    supportFile: "cypress/support/e2e.ts",
    video: true,
    screenshotOnRunFailure: true,
    defaultCommandTimeout: 10000,
    env: {
      // The Rust core, reachable directly (not through the Next.js BFF) so
      // "backend truth" reconciliation checks are independent of the proxy
      // code path each spec is exercising via the UI.
      CORE_URL: process.env.CORE_HTTP_URL || "http://localhost:3001",
    },
  },
});
