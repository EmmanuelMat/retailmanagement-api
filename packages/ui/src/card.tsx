import * as React from "react";
import { cn } from "./utils";

export const Card = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(({ className, ...props }, ref) => (
  <div ref={ref} className={cn("rounded-2xl border border-zinc-800 bg-zinc-900 p-4", className)} {...props} />
));
Card.displayName = "Card";
