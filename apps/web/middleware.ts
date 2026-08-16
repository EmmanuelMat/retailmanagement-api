import { NextRequest, NextResponse } from "next/server";

// One Next.js project serves two sites split by hostname: the customer
// product on the main domain, and the internal staff console on
// staff.<domain> (staff.localhost in dev). Both live under app/, but the
// staff pages sit behind a real /staff/* segment with its own root layout
// (app/staff/layout.tsx) so it gets a distinct theme/fonts - see that
// file for why. This middleware makes /staff invisible on the staff host
// (rewrite) and unreachable on the main host (redirect), so neither site
// leaks into the other's URL space.
const PUBLIC_CUSTOMER_PATHS = new Set(["/", "/login", "/registro", "/olvide-password", "/restablecer-password", "/recorrido"]);

export function middleware(req: NextRequest) {
  const host = req.headers.get("host") || "";
  const { pathname } = req.nextUrl;
  const isStaffHost = host.startsWith("staff.");

  if (isStaffHost) {
    if (pathname.startsWith("/staff")) return NextResponse.next();
    const url = req.nextUrl.clone();
    url.pathname = `/staff${pathname === "/" ? "" : pathname}`;
    return NextResponse.rewrite(url);
  }

  if (pathname.startsWith("/staff")) {
    return NextResponse.redirect(new URL("/", req.url));
  }

  // Guards every dashboard route: the login page already sets a `token`
  // cookie ("para middleware"), this is what actually reads it.
  if (PUBLIC_CUSTOMER_PATHS.has(pathname)) {
    return NextResponse.next();
  }

  const token = req.cookies.get("token")?.value;
  if (!token) {
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("redirect", pathname);
    return NextResponse.redirect(loginUrl);
  }
  return NextResponse.next();
}

export const config = {
  matcher: ["/((?!api|_next|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|webp)$).*)"],
};
