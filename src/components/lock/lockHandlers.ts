// Keydown and input handlers for POS Terminal Lock Screen
// F1.14 — Local POS PIN Authentication & Lock Screen

export interface LockScreenKeyActions {
  onDigit: (digit: string) => void
  onBackspace: () => void
  onClear: () => void
  onSubmit: () => void
}

export function handleLockScreenKeyDown(
  event: { key: string; ctrlKey?: boolean; metaKey?: boolean; altKey?: boolean; preventDefault: () => void },
  actions: LockScreenKeyActions,
  activeElement?: { tagName: string } | null,
): void {
  // Don't intercept if modifier keys are held
  if (event.ctrlKey || event.metaKey || event.altKey) return

  if (event.key >= '0' && event.key <= '9') {
    event.preventDefault()
    actions.onDigit(event.key)
  } else if (event.key === 'Backspace') {
    event.preventDefault()
    actions.onBackspace()
  } else if (event.key === 'Escape') {
    event.preventDefault()
    actions.onClear()
  } else if (event.key === 'Enter') {
    let activeEl: { tagName: string } | null = null
    if (activeElement !== undefined) {
      activeEl = activeElement
    } else if (typeof document !== 'undefined') {
      activeEl = document.activeElement
    }
    if (activeEl && (activeEl.tagName === 'BUTTON' || activeEl.tagName === 'A')) {
      return
    }
    event.preventDefault()
    actions.onSubmit()
  }
}
