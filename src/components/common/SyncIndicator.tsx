// Sync Indicator & Interactive Popover Component
// F1.17 — Offline / Online / Sync Status Indicator & SYNC_SPEC.md

import React, { useState, useEffect, useRef, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import type { SyncStatus } from '../../types/sync'
import {
  computeEffectiveSyncStatus,
  getSyncStatusLabel,
  getSyncLifecycleText,
  getSyncActionButtonText,
} from './syncHelpers'

export interface SyncIndicatorProps {
  isOnline: boolean
  isSyncing: boolean
  syncStatus?: SyncStatus
  pendingCount?: number
  lastSyncedAt?: string | null
  lastError?: string | null
  deviceId?: string | null
  onTriggerSync?: () => Promise<boolean | void>
}

interface ActionFeedback {
  message: string
  type: 'success' | 'error'
}

export const SyncIndicator: React.FC<SyncIndicatorProps> = ({
  isOnline,
  isSyncing,
  syncStatus,
  pendingCount = 0,
  lastSyncedAt = null,
  lastError = null,
  deviceId = null,
  onTriggerSync,
}) => {
  const { t, i18n } = useTranslation()
  const [isOpen, setIsOpen] = useState(false)
  const [isManualSyncing, setIsManualSyncing] = useState(false)
  const [actionFeedback, setActionFeedback] = useState<ActionFeedback | null>(null)

  const triggerRef = useRef<HTMLButtonElement>(null)
  const dialogRef = useRef<HTMLDialogElement>(null)

  const effectiveStatus = computeEffectiveSyncStatus(
    isOnline,
    isSyncing,
    isManualSyncing,
    syncStatus,
    lastError,
  )

  const statusLabel = getSyncStatusLabel(effectiveStatus, t)
  const lifecycleText = getSyncLifecycleText(effectiveStatus, pendingCount, t)
  const isBusy = isSyncing || isManualSyncing
  const actionButtonText = getSyncActionButtonText(isBusy, effectiveStatus, t)

  // Format last synced timestamp according to active locale
  const formattedLastSync = lastSyncedAt
    ? new Intl.DateTimeFormat(i18n.language || 'en', {
        dateStyle: 'short',
        timeStyle: 'medium',
      }).format(new Date(lastSyncedAt))
    : t('sync.neverSynced')

  const pendingLabel = pendingCount > 0 ? `, ${t('sync.pendingCountLabel', { count: pendingCount })}` : ''
  const triggerAriaLabel = `${t('sync.title')}: ${statusLabel}${pendingLabel}`
  const triggerTitle = pendingCount > 0 ? `${statusLabel} (${pendingCount})` : statusLabel
  const connectionBadgeClass = isOnline ? 'online' : 'offline'

  const handleClose = useCallback(() => {
    setIsOpen(false)
    triggerRef.current?.focus()
  }, [])

  // Manage native dialog visibility & events
  useEffect(() => {
    const dialogEl = dialogRef.current
    if (!dialogEl) return

    if (isOpen) {
      if (!dialogEl.open) {
        dialogEl.showModal()
      }
    } else if (dialogEl.open) {
      dialogEl.close()
    }
  }, [isOpen])

  // Handle Escape key and outside backdrop clicks
  useEffect(() => {
    if (!isOpen) return

    const dialogEl = dialogRef.current

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        handleClose()
      }
    }

    const handleClickOutside = (e: MouseEvent) => {
      if (
        dialogEl &&
        !dialogEl.contains(e.target as Node) &&
        triggerRef.current &&
        !triggerRef.current.contains(e.target as Node)
      ) {
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
    document.addEventListener('mousedown', handleClickOutside)
    dialogEl?.addEventListener('click', handleBackdropClick)

    return () => {
      document.removeEventListener('keydown', handleKeyDown)
      document.removeEventListener('mousedown', handleClickOutside)
      dialogEl?.removeEventListener('click', handleBackdropClick)
    }
  }, [isOpen, handleClose])

  const handleToggle = () => {
    setIsOpen((prev) => !prev)
  }

  const handleSyncClick = useCallback(async () => {
    if (!onTriggerSync || !isOnline || isBusy) return

    setIsManualSyncing(true)
    setActionFeedback(null)
    try {
      const result = await onTriggerSync()
      if (result === false) {
        setActionFeedback({
          message: t('sync.syncError'),
          type: 'error',
        })
      } else {
        setActionFeedback({
          message: t('sync.syncSuccess'),
          type: 'success',
        })
      }
    } catch {
      setActionFeedback({
        message: t('sync.syncError'),
        type: 'error',
      })
    } finally {
      setIsManualSyncing(false)
    }
  }, [onTriggerSync, isOnline, isBusy, t])

  return (
    <div className="sync-indicator-wrapper" data-testid="sync-indicator-wrapper">
      {/* Indicator Trigger Badge */}
      <button
        ref={triggerRef}
        type="button"
        className="sync-badge-trigger"
        onClick={handleToggle}
        aria-haspopup="dialog"
        aria-expanded={isOpen}
        aria-label={triggerAriaLabel}
        title={triggerTitle}
        data-testid="sync-indicator-trigger"
      >
        <span
          className={`sync-dot sync-dot--${effectiveStatus}`}
          aria-hidden="true"
          data-testid="sync-status-dot"
        />
        <span>{statusLabel}</span>
        {pendingCount > 0 && (
          <span
            className="sync-pending-badge"
            title={t('sync.pendingCountLabel', { count: pendingCount })}
            data-testid="sync-pending-badge"
          >
            {pendingCount}
          </span>
        )}
      </button>

      {/* Screen Reader Live Region */}
      <output className="sr-only" aria-live="polite" data-testid="sync-live-region">
        {statusLabel}
      </output>

      {/* Popover Details Dialog */}
      {isOpen && (
        <dialog
          ref={dialogRef}
          aria-labelledby="sync-popover-heading"
          className="sync-popover-card"
          data-testid="sync-popover-card"
          onCancel={(e) => {
            e.preventDefault()
            handleClose()
          }}
        >
          <div className="sync-popover-header">
            <h3 id="sync-popover-heading" className="sync-popover-title">
              {t('sync.title')}
            </h3>
            <button
              type="button"
              className="sync-popover-close-btn"
              onClick={handleClose}
              aria-label={t('sync.closeModal')}
              data-testid="sync-popover-close-btn"
            >
              ✕
            </button>
          </div>

          <div className="sync-popover-list">
            {/* Connection Status Row */}
            <div className="sync-popover-row">
              <span className="sync-popover-label">{t('sync.connectionStatus')}</span>
              <span className="sync-popover-value">
                <span
                  className={`sync-status-badge sync-status-badge--${connectionBadgeClass}`}
                >
                  {isOnline ? t('status.online') : t('status.offline')}
                </span>
              </span>
            </div>

            {/* Sync Lifecycle State Row */}
            <div className="sync-popover-row">
              <span className="sync-popover-label">{t('sync.syncLifecycle')}</span>
              <span className="sync-popover-value">
                <span className={`sync-status-badge sync-status-badge--${effectiveStatus}`}>
                  {lifecycleText}
                </span>
              </span>
            </div>

            {/* Pending Outbox Queue Row */}
            <div className="sync-popover-row">
              <span className="sync-popover-label">{t('sync.pendingChanges')}</span>
              <span className="sync-popover-value" data-testid="sync-pending-count-value">
                {pendingCount}
              </span>
            </div>

            {/* Last Synced Row */}
            <div className="sync-popover-row">
              <span className="sync-popover-label">{t('sync.lastSynced')}</span>
              <span className="sync-popover-value" data-testid="sync-last-synced-value">
                {formattedLastSync}
              </span>
            </div>

            {/* Terminal Device ID */}
            {deviceId && (
              <div className="sync-popover-row">
                <span className="sync-popover-label">{t('sync.terminalId')}</span>
                <span className="sync-popover-value">
                  <code>{deviceId}</code>
                </span>
              </div>
            )}
          </div>

          {/* Last Error Message Banner */}
          {lastError && (
            <div className="sync-popover-error" role="alert" data-testid="sync-error-banner">
              <span>{lastError}</span>
            </div>
          )}

          {/* Offline Non-Blocking Notice */}
          {!isOnline && (
            <div className="sync-popover-error" style={{ borderColor: 'var(--color-warning)' }}>
              <span>{t('sync.offlineNotice')}</span>
            </div>
          )}

          {/* Action Feedback Banner */}
          {actionFeedback && actionFeedback.type === 'error' && (
            <div
              className="sync-popover-error"
              role="alert"
              data-testid="sync-action-feedback"
            >
              <span>{actionFeedback.message}</span>
            </div>
          )}

          {actionFeedback && actionFeedback.type === 'success' && (
            <output
              className="sync-feedback-success"
              data-testid="sync-action-feedback"
            >
              <span>{actionFeedback.message}</span>
            </output>
          )}

          {/* Actions */}
          <div className="sync-popover-actions">
            <button
              type="button"
              className="btn btn--primary btn--sm sync-action-btn"
              onClick={handleSyncClick}
              disabled={!isOnline || isBusy}
              data-testid="sync-now-btn"
            >
              {actionButtonText}
            </button>
          </div>
        </dialog>
      )}
    </div>
  )
}
