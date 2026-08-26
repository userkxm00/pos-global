// Confirmation and Audit Reason Dialog Component
// F1.18 — Authorization and Error-State UX & UI_CLOUD_EXECUTION_PLAN.md Rule 4

import React, { useState, useEffect, useRef, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { AlertIcon, WarningTriangleIcon } from './Icons'

export interface ConfirmationDialogProps {
  isOpen: boolean
  title: string
  description: string
  confirmLabel?: string
  cancelLabel?: string
  isDestructive?: boolean
  requireReason?: boolean
  reasonPlaceholder?: string
  onConfirm: (reason?: string) => void | Promise<void>
  onCancel: () => void
}

export const ConfirmationDialog: React.FC<ConfirmationDialogProps> = ({
  isOpen,
  title,
  description,
  confirmLabel,
  cancelLabel,
  isDestructive = false,
  requireReason = false,
  reasonPlaceholder,
  onConfirm,
  onCancel,
}) => {
  const { t } = useTranslation()
  const [reason, setReason] = useState('')
  const [reasonError, setReasonError] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)

  const dialogRef = useRef<HTMLDialogElement>(null)
  const confirmBtnRef = useRef<HTMLButtonElement>(null)
  const reasonInputRef = useRef<HTMLInputElement>(null)

  // Reset internal state when opened
  useEffect(() => {
    if (isOpen) {
      setReason('')
      setReasonError(null)
      setIsSubmitting(false)
    }
  }, [isOpen])

  // Native dialog modal lifecycle
  useEffect(() => {
    const dialogEl = dialogRef.current
    if (!dialogEl) return

    if (isOpen) {
      if (!dialogEl.open) {
        dialogEl.showModal()
      }
      if (requireReason) {
        reasonInputRef.current?.focus()
      } else {
        confirmBtnRef.current?.focus()
      }
    } else if (dialogEl.open) {
      dialogEl.close()
    }
  }, [isOpen, requireReason])

  const handleClose = useCallback(() => {
    if (isSubmitting) return
    onCancel()
  }, [isSubmitting, onCancel])

  // Escape key and backdrop click listener
  useEffect(() => {
    if (!isOpen) return

    const dialogEl = dialogRef.current

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        handleClose()
      }
    }

    const handleBackdropClick = (e: MouseEvent) => {
      if (!dialogEl) return
      const rect = dialogEl.getBoundingClientRect()
      const isInside =
        e.clientX >= rect.left &&
        e.clientX <= rect.right &&
        e.clientY >= rect.top &&
        e.clientY <= rect.bottom
      if (!isInside) {
        handleClose()
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    dialogEl?.addEventListener('click', handleBackdropClick)

    return () => {
      document.removeEventListener('keydown', handleKeyDown)
      dialogEl?.removeEventListener('click', handleBackdropClick)
    }
  }, [isOpen, handleClose])

  const handleConfirmClick = async () => {
    if (requireReason && !reason.trim()) {
      setReasonError(t('confirmation.reasonRequired'))
      reasonInputRef.current?.focus()
      return
    }

    setIsSubmitting(true)
    try {
      await onConfirm(reason.trim() || undefined)
    } finally {
      setIsSubmitting(false)
    }
  }

  if (!isOpen) {
    return null
  }

  const effectiveConfirmLabel =
    confirmLabel || (isDestructive ? t('confirmation.delete') : t('confirmation.confirm'))
  const effectiveCancelLabel = cancelLabel || t('confirmation.cancel')

  return (
    <dialog
      ref={dialogRef}
      className={`confirmation-dialog ${isDestructive ? 'confirmation-dialog--destructive' : ''}`}
      aria-labelledby="confirmation-dialog-title"
      aria-describedby="confirmation-dialog-desc"
      data-testid="confirmation-dialog"
      onCancel={(e) => {
        e.preventDefault()
        handleClose()
      }}
    >
      <div className="confirmation-dialog__header">
        <div className="confirmation-dialog__icon" aria-hidden="true">
          {isDestructive ? <AlertIcon size={24} /> : <WarningTriangleIcon size={24} />}
        </div>
        <h3 id="confirmation-dialog-title" className="confirmation-dialog__title">
          {title}
        </h3>
      </div>

      <div className="confirmation-dialog__body">
        <p id="confirmation-dialog-desc" className="confirmation-dialog__description">
          {description}
        </p>

        {requireReason && (
          <div className="confirmation-dialog__reason-field">
            <label htmlFor="confirmation-reason-input" className="form-label">
              {t('confirmation.reasonLabel')}{' '}
              <span className="required-asterisk" aria-hidden="true">
                *
              </span>
            </label>
            <input
              ref={reasonInputRef}
              id="confirmation-reason-input"
              type="text"
              className={`form-input ${reasonError ? 'form-input--error' : ''}`}
              placeholder={reasonPlaceholder || t('confirmation.reasonPlaceholder')}
              value={reason}
              onChange={(e) => {
                setReason(e.target.value)
                if (reasonError) setReasonError(null)
              }}
              disabled={isSubmitting}
              data-testid="confirmation-reason-input"
            />
            {reasonError && (
              <p className="form-error-text" role="alert" data-testid="confirmation-reason-error">
                {reasonError}
              </p>
            )}
          </div>
        )}
      </div>

      <div className="confirmation-dialog__footer">
        <button
          type="button"
          className="btn btn--secondary"
          onClick={handleClose}
          disabled={isSubmitting}
          data-testid="confirmation-cancel-btn"
        >
          {effectiveCancelLabel}
        </button>
        <button
          ref={confirmBtnRef}
          type="button"
          className={`btn ${isDestructive ? 'btn--danger' : 'btn--primary'}`}
          onClick={handleConfirmClick}
          disabled={isSubmitting}
          data-testid="confirmation-confirm-btn"
        >
          {isSubmitting ? t('status.processing') : effectiveConfirmLabel}
        </button>
      </div>
    </dialog>
  )
}
