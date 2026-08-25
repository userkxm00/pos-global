// Inactivity timeout hook and tracker for terminal lock screen orchestration
// F1.14 — Local PIN and Lock screen

import { useEffect, useRef } from 'react'

export const DEFAULT_INACTIVITY_TIMEOUT_MS = 15 * 60 * 1000 // 15 minutes default

export interface UseInactivityTimeoutOptions {
  onTimeout: () => void
  timeoutMs?: number
  isEnabled?: boolean
}

export interface InactivityTracker {
  reset: () => void
  cleanup: () => void
}

export const ACTIVITY_EVENTS: readonly (keyof WindowEventMap)[] = [
  'mousemove',
  'mousedown',
  'keydown',
  'touchstart',
  'scroll',
] as const

export function createInactivityTracker(options: UseInactivityTimeoutOptions): InactivityTracker {
  const { onTimeout, timeoutMs = DEFAULT_INACTIVITY_TIMEOUT_MS, isEnabled = true } = options
  let timer: ReturnType<typeof setTimeout> | null = null

  const reset = () => {
    if (timer) {
      clearTimeout(timer)
      timer = null
    }
    if (isEnabled && timeoutMs > 0) {
      timer = setTimeout(() => {
        onTimeout()
      }, timeoutMs)
    }
  }

  const cleanup = () => {
    if (timer) {
      clearTimeout(timer)
      timer = null
    }
    if (typeof window !== 'undefined') {
      for (const eventName of ACTIVITY_EVENTS) {
        window.removeEventListener(eventName, reset)
      }
    }
  }

  if (isEnabled && timeoutMs > 0) {
    reset()
    if (typeof window !== 'undefined') {
      for (const eventName of ACTIVITY_EVENTS) {
        window.addEventListener(eventName, reset, { passive: true })
      }
    }
  }

  return { reset, cleanup }
}

export function useInactivityTimeout(options: UseInactivityTimeoutOptions): void {
  const { onTimeout, timeoutMs = DEFAULT_INACTIVITY_TIMEOUT_MS, isEnabled = true } = options
  const onTimeoutRef = useRef(onTimeout)

  useEffect(() => {
    onTimeoutRef.current = onTimeout
  }, [onTimeout])

  useEffect(() => {
    if (!isEnabled || timeoutMs <= 0) return
    const tracker = createInactivityTracker({
      onTimeout: () => onTimeoutRef.current(),
      timeoutMs,
      isEnabled,
    })
    return () => {
      tracker.cleanup()
    }
  }, [isEnabled, timeoutMs])
}
