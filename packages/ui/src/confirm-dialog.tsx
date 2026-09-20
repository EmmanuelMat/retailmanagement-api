"use client";

import * as React from "react";
import { Button } from "./button";
import { Dialog } from "./dialog";

export interface ConfirmDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: React.ReactNode;
  description?: React.ReactNode;
  confirmLabel: string;
  destructive?: boolean;
  busy?: boolean;
}

// Confirmation step for irreversible or money-moving actions. While `busy`
// the dialog can't be dismissed, so a second click can't fire the action twice.
export function ConfirmDialog({ open, onClose, onConfirm, title, description, confirmLabel, destructive, busy }: ConfirmDialogProps) {
  return (
    <Dialog open={open} onClose={busy ? () => {} : onClose} title={title} className="max-w-md" testId="confirm-dialog">
      {description && <div className="text-sm text-muted-foreground space-y-2">{description}</div>}
      <div className="flex justify-end gap-2 mt-6">
        <Button type="button" variant="outline" disabled={busy} onClick={onClose} data-testid="confirm-dialog-cancel">
          Cancelar
        </Button>
        <Button
          type="button"
          variant={destructive ? "destructive" : "default"}
          disabled={busy}
          onClick={onConfirm}
          data-testid="confirm-dialog-submit"
        >
          {busy ? "Procesando..." : confirmLabel}
        </Button>
      </div>
    </Dialog>
  );
}
