// Toast Notification Context & Hook
// F1.18 — Authorization and Error-State UX & UI_SPEC.md

import React, { createContext, useContext, useState, useCallback, useMemo, useRef } from 'react'
import type { ToastMessage, ToastVariant } from '../types/feedback'

export interface ToastContextType {
  toasts: ToastMessage[]
  showToast: (toast: Omit<ToastMessage, 'id'>) => string
  showError: (message: string, title?: string) => string
  showWarning: (message: string, title?: string) => string
  showSuccess: (message: string, title?: string) => string
  showInfo: (message: string, title?: string) => string
  dismissToast: (id: string) => void
  clearAllToasts: () => void
}

const ToastContext = createContext<ToastContextType | null>(null)

export interface ToastProviderProps {
  children: React.ReactNode
  defaultDurationMs?: number
}

const DEFAULT_TOAST_DURATION = 5000

export const ToastProvider: React.FC<ToastProviderProps> = ({
  children,
  defaultDurationMs = DEFAULT_TOAST_DURATION,
}) => {
  const [toasts, setToasts] = useState<ToastMessage[]>([])
  const counterRef = useRef<number>(0)

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }, [])

  const clearAllToasts = useCallback(() => {
    setToasts([])
  }, [])

  const showToast = useCallback(
    (toastInput: Omit<ToastMessage, 'id'>): string => {
      counterRef.current += 1
      const id = `toast_${Date.now()}_${counterRef.current}`
      const newToast: ToastMessage = {
        id,
        durationMs: defaultDurationMs,
        ...toastInput,
      }
      setToasts((prev) => [...prev, newToast])
      return id
    },
    [defaultDurationMs],
  )

  const showError = useCallback(
    (message: string, title?: string): string => {
      return showToast({ variant: 'error', message, title })
    },
    [showToast],
  )

  const showWarning = useCallback(
    (message: string, title?: string): string => {
      return showToast({ variant: 'warning', message, title })
    },
    [showToast],
  )

  const showSuccess = useCallback(
    (message: string, title?: string): string => {
      return showToast({ variant: 'success', message, title })
    },
    [showToast],
  )

  const showInfo = useCallback(
    (message: string, title?: string): string => {
      return showToast({ variant: 'info', message, title })
    },
    [showToast],
  )

  const value = useMemo<ToastContextType>(
    () => ({
      toasts,
      showToast,
      showError,
      showWarning,
      showSuccess,
      showInfo,
      dismissToast,
      clearAllToasts,
    }),
    [toasts, showToast, showError, showWarning, showSuccess, showInfo, dismissToast, clearAllToasts],
  )

  return <ToastContext.Provider value={value}>{children}</ToastContext.Provider>
}

export function useToast(): ToastContextType {
  const context = useContext(ToastContext)
  if (!context) {
    throw new Error('useToast must be used within a ToastProvider')
  }
  return context
}
