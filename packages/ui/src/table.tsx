import * as React from "react";
import { ArrowDown, ArrowUp } from "lucide-react";
import { cn } from "./utils";

export const Table = React.forwardRef<HTMLTableElement, React.TableHTMLAttributes<HTMLTableElement>>(({ className, ...props }, ref) => (
  <div className="w-full overflow-x-auto">
    <table ref={ref} className={cn("w-full text-sm border-collapse", className)} {...props} />
  </div>
));
Table.displayName = "Table";

export const TableHeader = React.forwardRef<HTMLTableSectionElement, React.HTMLAttributes<HTMLTableSectionElement>>(({ className, ...props }, ref) => (
  <thead ref={ref} className={cn("border-b border-border bg-muted/40", className)} {...props} />
));
TableHeader.displayName = "TableHeader";

export const TableBody = React.forwardRef<HTMLTableSectionElement, React.HTMLAttributes<HTMLTableSectionElement>>(({ className, ...props }, ref) => (
  <tbody ref={ref} className={cn("divide-y divide-border", className)} {...props} />
));
TableBody.displayName = "TableBody";

export const TableRow = React.forwardRef<HTMLTableRowElement, React.HTMLAttributes<HTMLTableRowElement>>(({ className, ...props }, ref) => (
  <tr ref={ref} className={cn("hover:bg-muted/50 transition-colors", className)} {...props} />
));
TableRow.displayName = "TableRow";

export const TableHead = React.forwardRef<HTMLTableCellElement, React.ThHTMLAttributes<HTMLTableCellElement>>(({ className, ...props }, ref) => (
  <th ref={ref} className={cn("h-10 px-3 text-left align-middle font-bold uppercase tracking-wide text-muted-foreground text-[11px]", className)} {...props} />
));
TableHead.displayName = "TableHead";

export const TableCell = React.forwardRef<HTMLTableCellElement, React.TdHTMLAttributes<HTMLTableCellElement>>(({ className, ...props }, ref) => (
  <td ref={ref} className={cn("px-3 py-3 align-middle", className)} {...props} />
));
TableCell.displayName = "TableCell";

export interface SortableTableHeadProps extends React.ThHTMLAttributes<HTMLTableCellElement> {
  column: string;
  activeSort: string | null;
  sortDir: "asc" | "desc";
  onSort: (column: string) => void;
}

export const SortableTableHead = React.forwardRef<HTMLTableCellElement, SortableTableHeadProps>(
  ({ column, activeSort, sortDir, onSort, children, className, ...props }, ref) => (
    <TableHead
      ref={ref}
      className={cn("cursor-pointer select-none hover:text-foreground", className)}
      onClick={() => onSort(column)}
      {...props}
    >
      <span className="inline-flex items-center gap-1">
        {children}
        {activeSort === column &&
          (sortDir === "asc" ? <ArrowUp className="h-3 w-3" /> : <ArrowDown className="h-3 w-3" />)}
      </span>
    </TableHead>
  )
);
SortableTableHead.displayName = "SortableTableHead";
