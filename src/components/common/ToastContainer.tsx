// Toast Notification Container Component
// F1.18 — Authorization and Error-State UX & UI_SPEC.md

import React, { useEffect, useState, useRef, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useToast } from '../../context/ToastContext'
import type { ToastMessage, ToastVariant } from '../../types/feedback'
import {
  AlertIcon,
  CheckCircleIcon,
  InfoCircleIcon,
  WarningTriangleIcon,
} from './Icons'

function getToastIcon(variant: ToastVariant): React.ReactNode {
  switch (variant) {
    case 'error':
      return <AlertIcon size={20} />
    case 'warning':
      return <WarningTriangleIcon size={20} />
    case 'success':
      return <CheckCircleIcon size={20} />
    case 'info':
    default:
      return <InfoCircleIcon size={20} />
  }
}

interface ToastItemProps {
  toast: ToastMessage
  onDismiss: (id: string) => void
}

const ToastItem: React.FC<ToastItemProps> = ({ toast, onDismiss }) => {
  const { t } = useTranslation()
  const [isPaused, setIsPaused] = useState(false)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const remainingTimeRef = useRef<number>(toast.durationMs ?? 5000)
  const startTimeRef = useRef<number>(Date.now())

  const handleDismiss = useCallback(() => {
    onDismiss(toast.id)
  }, [onDismiss, toast.id])

  useEffect(() => {
    if (!toast.durationMs || toast.durationMs <= 0) return

    if (!isPaused) {
      startTimeRef.current = Date.now()
      timerRef.current = setTimeout(() => {
        handleDismiss()
      }, remainingTimeRef.current)
    } else if (timerRef.current) {
      clearTimeout(timerRef.current)
      const elapsed = Date.now() - startTimeRef.current
      remainingTimeRef.current = Math.max(0, remainingTimeRef.current - elapsed)
    }

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current)
      }
    }
  }, [isPaused, toast.durationMs, handleDismiss])

  const handleMouseEnter = () => setIsPaused(true)
  const handleMouseLeave = () => setIsPaused(false)
  const handleFocus = () => setIsPaused(true)
  const handleBlur = () => setIsPaused(false)

  const isAlert = toast.variant === 'error' || toast.variant === 'warning'

  return (
    <div
      className={`toast-item toast-item--${toast.variant}`}
      role={isAlert ? 'alert' : 'status'}
      aria-live={isAlert ? 'assertive' : 'polite'}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onFocus={handleFocus}
      onBlur={handleBlur}
      data-testid={`toast-${toast.id}`}
    >
      <div className="toast-item__icon" aria-hidden="true">
        {getToastIcon(toast.variant)}
      </div>

      <div className="toast-item__content">
        {toast.title && <h4 className="toast-item__title">{toast.title}</h4>}
        <p className="toast-item__message">{toast.message}</p>
        {toast.actionLabel && toast.onAction && (
          <button
            type="button"
            className="toast-item__action-btn"
            onClick={() => {
              toast.onAction?.()
              handleDismiss()
            }}
          >
            {toast.actionLabel}
          </button>
        )}
      </div>

      <button
        type="button"
        className="toast-item__close-btn"
        onClick={handleDismiss}
        aria-label={t('toasts.dismiss')}
        data-testid={`toast-close-${toast.id}`}
      >
        ✕
      </button>
    </div>
  )
}

export const ToastContainer: React.FC = () => {
  const { toasts, dismissToast } = useToast()

  if (toasts.length === 0) {
    return null
  }

  return (
    <section
      className="toast-container"
      aria-label="Notifications"
      data-testid="toast-container"
    >
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onDismiss={dismissToast} />
      ))}
    </section>
  )
}
