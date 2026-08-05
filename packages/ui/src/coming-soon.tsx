import * as React from "react";
import { Card, CardContent } from "./card";
import { cn } from "./utils";

export function ComingSoon({
  title,
  description,
  className,
}: {
  title: string;
  description?: string;
  className?: string;
}) {
  return (
    <Card className={cn("max-w-lg", className)}>
      <CardContent className="py-10 text-center">
        <p className="text-xs font-semibold text-primary">Próximamente</p>
        <h2 className="text-lg font-bold mt-1">{title}</h2>
        {description && <p className="text-sm text-muted-foreground mt-2">{description}</p>}
      </CardContent>
    </Card>
  );
}
