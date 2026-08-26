// Pure helpers for ConfirmationDialog
// F1.18 — Authorization and Error-State UX

export function isBackdropClick(
  target: unknown,
  dialogEl: unknown,
  clientX: number,
  clientY: number,
  detail: number,
  rect: { left: number; right: number; top: number; bottom: number } | DOMRect,
): boolean {
  // If click was on a child element (e.g. Confirm or Cancel button), never treat as backdrop
  if (target !== dialogEl) return false

  // Synthetic events from keyboard activation on buttons often have clientX=0, clientY=0, detail=0
  if (clientX === 0 && clientY === 0 && detail === 0) return false

  return (
    clientX < rect.left ||
    clientX > rect.right ||
    clientY < rect.top ||
    clientY > rect.bottom
  )
}

export function validateConfirmationReason(
  requireReason: boolean,
  reason: string | undefined | null,
): boolean {
  if (!requireReason) return true
  return Boolean(reason && reason.trim().length > 0)
}
