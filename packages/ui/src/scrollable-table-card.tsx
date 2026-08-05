import * as React from "react";
import { Card, CardContent } from "./card";

export interface ScrollableTableCardProps {
  loading: boolean;
  error: string | null;
  isEmpty: boolean;
  emptyMessage: React.ReactNode;
  emptyIcon?: React.ReactNode;
  pagination: React.ReactNode;
  children: React.ReactNode;
  /**
   * Vertical space reserved above the card (page header + filters row).
   * Defaults to the same `100vh - 104px` convention already used by the
   * POS screen's fixed cart panel, so only the table body scrolls while
   * the page title/filters/pagination stay in place.
   */
  maxHeight?: string;
}

/**
 * Shared table shell: bounded height with an independently scrolling body,
 * plus consistent loading/empty/error states and a pinned pagination footer.
 * Used by every server-paginated table (Productos, Ventas, Compras,
 * Movimientos, Auditoría) so they share one scroll/empty/error behavior
 * instead of each page hand-rolling it.
 */
export function ScrollableTableCard({
  loading,
  error,
  isEmpty,
  emptyMessage,
  emptyIcon,
  pagination,
  children,
  maxHeight = "calc(100vh - 104px)",
}: ScrollableTableCardProps) {
  return (
    <>
      {error && (
        <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm mb-4">
          {error}
        </div>
      )}
      <Card className="flex flex-col" style={{ maxHeight }}>
        <CardContent className="p-0 flex-1 min-h-0 overflow-y-auto">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : isEmpty ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              {emptyIcon}
              {emptyMessage}
            </div>
          ) : (
            children
          )}
        </CardContent>
        {!loading && !isEmpty && (
          <div className="px-3 border-t border-border shrink-0">{pagination}</div>
        )}
      </Card>
    </>
  );
}
