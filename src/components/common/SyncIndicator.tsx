import React from 'react'
import { useTranslation } from 'react-i18next'

export interface SyncIndicatorProps {
  isOnline: boolean
  isSyncing: boolean
}

export const SyncIndicator: React.FC<SyncIndicatorProps> = ({ isOnline, isSyncing }) => {
  const { t } = useTranslation()

  let statusKey = 'status.online'
  let indicatorClass = 'status-indicator status-indicator--online'

  if (!isOnline) {
    statusKey = 'status.offline'
    indicatorClass = 'status-indicator status-indicator--offline'
  } else if (isSyncing) {
    statusKey = 'status.syncing'
    indicatorClass = 'status-indicator status-indicator--syncing'
  }

  const label = t(statusKey)

  return (
    <div
      className="tenant-badge"
      aria-live="polite"
      title={label}
      data-testid="sync-indicator"
    >
      <span className={indicatorClass} aria-hidden="true" />
      <span>{label}</span>
    </div>
  )
}
