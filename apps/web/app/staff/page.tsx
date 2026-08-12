"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

export default function RootPage() {
  const router = useRouter();
  useEffect(() => {
    const secret = localStorage.getItem("vendorSecret");
    router.replace((secret ? "/tenants" : "/login") as any);
  }, [router]);
  return null;
}
