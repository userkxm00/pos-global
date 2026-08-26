// Sync Indicator & Interactive Popover Component
// F1.17 — Offline / Online / Sync Status Indicator & SYNC_SPEC.md

import React, { useState, useEffect, useRef, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import type { SyncStatus } from '../../types/sync'

export interface SyncIndicatorProps {
  isOnline: boolean
  isSyncing: boolean
  syncStatus?: SyncStatus
  pendingCount?: number
  lastSyncedAt?: string | null
  lastError?: string | null
  deviceId?: string | null
  onTriggerSync?: () => Promise<void>
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
  const [actionFeedback, setActionFeedback] = useState<string | null>(null)

  const triggerRef = useRef<HTMLButtonElement>(null)
  const popoverRef = useRef<HTMLDivElement>(null)

  // Derive active sync state
  const effectiveStatus: SyncStatus = !isOnline
    ? 'offline'
    : isSyncing || isManualSyncing
      ? 'syncing'
      : syncStatus === 'error' || lastError
        ? 'error'
        : 'online'

  // Localized status label
  const statusLabel =
    effectiveStatus === 'offline'
      ? t('status.offline')
      : effectiveStatus === 'syncing'
        ? t('status.syncing')
        : effectiveStatus === 'error'
          ? t('sync.syncFailed')
          : t('status.online')

  // Format last synced timestamp according to active locale
  const formattedLastSync = lastSyncedAt
    ? new Intl.DateTimeFormat(i18n.language || 'en', {
        dateStyle: 'short',
        timeStyle: 'medium',
      }).format(new Date(lastSyncedAt))
    : t('sync.neverSynced')

  // Keyboard navigation & click outside listener for popover dismissal
  useEffect(() => {
    if (!isOpen) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsOpen(false)
        triggerRef.current?.focus()
      }
    }

    const handleClickOutside = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node) &&
        triggerRef.current &&
        !triggerRef.current.contains(e.target as Node)
      ) {
        setIsOpen(false)
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    document.addEventListener('mousedown', handleClickOutside)

    return () => {
      document.removeEventListener('keydown', handleKeyDown)
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [isOpen])

  const handleToggle = () => {
    setIsOpen((prev) => !prev)
  }

  const handleSyncClick = useCallback(async () => {
    if (!onTriggerSync || !isOnline || isSyncing || isManualSyncing) return

    setIsManualSyncing(true)
    setActionFeedback(null)
    try {
      await onTriggerSync()
      setActionFeedback(t('sync.syncSuccess'))
    } catch {
      setActionFeedback(t('sync.syncError'))
    } finally {
      setIsManualSyncing(false)
    }
  }, [onTriggerSync, isOnline, isSyncing, isManualSyncing, t])

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
        aria-label={`${t('sync.title')}: ${statusLabel}${pendingCount > 0 ? `, ${t('sync.pendingCountLabel', { count: pendingCount })}` : ''}`}
        title={`${statusLabel}${pendingCount > 0 ? ` (${pendingCount})` : ''}`}
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
      <div className="sr-only" role="status" aria-live="polite" data-testid="sync-live-region">
        {statusLabel}
      </div>

      {/* Popover Details Card */}
      {isOpen && (
        <div
          ref={popoverRef}
          role="dialog"
          aria-modal="false"
          aria-labelledby="sync-popover-heading"
          className="sync-popover-card"
          data-testid="sync-popover-card"
        >
          <div className="sync-popover-header">
            <h3 id="sync-popover-heading" className="sync-popover-title">
              {t('sync.title')}
            </h3>
            <button
              type="button"
              className="sync-popover-close-btn"
              onClick={() => {
                setIsOpen(false)
                triggerRef.current?.focus()
              }}
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
                  className={`sync-status-badge sync-status-badge--${isOnline ? 'online' : 'offline'}`}
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
                  {effectiveStatus === 'syncing'
                    ? t('sync.syncing')
                    : effectiveStatus === 'error'
                      ? t('sync.syncFailed')
                      : pendingCount > 0
                        ? t('status.pendingChanges_other', { count: pendingCount })
                        : t('sync.synced')}
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
          {actionFeedback && (
            <div className="sync-popover-row" style={{ color: 'var(--color-success)' }}>
              <span>{actionFeedback}</span>
            </div>
          )}

          {/* Actions */}
          <div className="sync-popover-actions">
            <button
              type="button"
              className="btn btn--primary btn--sm sync-action-btn"
              onClick={handleSyncClick}
              disabled={!isOnline || isSyncing || isManualSyncing}
              data-testid="sync-now-btn"
            >
              {isSyncing || isManualSyncing
                ? t('sync.syncingAction')
                : effectiveStatus === 'error'
                  ? t('sync.retrySync')
                  : t('sync.syncNow')}
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
