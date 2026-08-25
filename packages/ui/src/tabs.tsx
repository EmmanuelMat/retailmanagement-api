"use client";

import * as React from "react";
import { cn } from "./utils";

export interface TabItem {
  value: string;
  label: React.ReactNode;
  /** Small count/indicator rendered after the label, e.g. item count. */
  badge?: React.ReactNode;
}

export interface TabsProps {
  items: TabItem[];
  value: string;
  onChange: (value: string) => void;
  className?: string;
}

/** Controlled tab bar - no content rendering, just the strip of buttons.
 * Pair with plain conditional rendering for the panels (see
 * ordenes-servicio/[id]/page.tsx for the reference usage). */
export function Tabs({ items, value, onChange, className }: TabsProps) {
  return (
    <div className={cn("flex gap-1 overflow-x-auto border-b border-border", className)} role="tablist">
      {items.map((item) => {
        const active = item.value === value;
        return (
          <button
            key={item.value}
            type="button"
            role="tab"
            aria-selected={active}
            onClick={() => onChange(item.value)}
            className={cn(
              "shrink-0 px-3.5 py-2.5 text-sm font-medium border-b-2 -mb-px transition-colors",
              active
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            )}
          >
            {item.label}
            {item.badge !== undefined && <span className="ml-1.5 opacity-60">{item.badge}</span>}
          </button>
        );
      })}
    </div>
  );
}
