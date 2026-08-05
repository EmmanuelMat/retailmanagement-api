import { NextRequest, NextResponse } from "next/server";

// Guards every dashboard route: the login page already sets a `token` cookie
// ("para middleware"), this is what actually reads it. "/" is the public
// marketing page and stays out of the gate.
export function middleware(req: NextRequest) {
  const token = req.cookies.get("token")?.value;
  if (!token) {
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("redirect", req.nextUrl.pathname);
    return NextResponse.redirect(loginUrl);
  }
  return NextResponse.next();
}

export const config = {
  matcher: ["/((?!$|login|registro|olvide-password|restablecer-password|api|_next|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|webp)$).*)"],
};
