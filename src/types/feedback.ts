// Feedback & Notification types
// F1.18 — Authorization and Error-State UX & UI_SPEC.md

export type ToastVariant = 'error' | 'warning' | 'info' | 'success'

export interface ToastMessage {
  id: string
  variant: ToastVariant
  title?: string
  message: string
  durationMs?: number
  actionLabel?: string
  onAction?: () => void
}

export interface ConfirmationDialogOptions {
  title: string
  description: string
  confirmLabel?: string
  cancelLabel?: string
  isDestructive?: boolean
  requireReason?: boolean
  reasonPlaceholder?: string
  onConfirm: (reason?: string) => void | Promise<void>
  onCancel?: () => void
}
