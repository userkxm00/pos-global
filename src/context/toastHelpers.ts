// Pure helpers for ToastContext
// F1.18 — Authorization and Error-State UX

export const DEFAULT_TOAST_DURATION = 5000

export function resolveToastDuration(
  inputDuration: number | undefined,
  defaultDurationMs: number = DEFAULT_TOAST_DURATION,
): number {
  return inputDuration ?? defaultDurationMs
}
