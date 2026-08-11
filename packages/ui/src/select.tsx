import * as React from "react";
import { cn } from "./utils";

// Custom chevron via background-image instead of the browser-native arrow —
// every <select> across the app gets the exact same icon position/size
// regardless of width, so nothing needs to be fixed per-page. The stroke
// color is a neutral mid-gray chosen to read on both the light and dark
// --surface tokens (data-URI SVGs can't reference CSS custom properties).
const CHEVRON_BG =
  "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%2382807a' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E\")";

export const Select = React.forwardRef<HTMLSelectElement, React.SelectHTMLAttributes<HTMLSelectElement>>(
  ({ className, children, style, ...props }, ref) => (
    <select
      ref={ref}
      className={cn(
        "flex h-10 w-full appearance-none rounded-md border border-border bg-surface bg-no-repeat px-3 pr-9 py-2 text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 focus-visible:border-primary/50 disabled:opacity-50",
        className
      )}
      style={{
        backgroundImage: CHEVRON_BG,
        backgroundPosition: "right 0.6rem center",
        backgroundSize: "16px 16px",
        ...style,
      }}
      {...props}
    >
      {children}
    </select>
  )
);
Select.displayName = "Select";
