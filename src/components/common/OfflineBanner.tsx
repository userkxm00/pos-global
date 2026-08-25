import React from 'react'
import { useTranslation } from 'react-i18next'
import { OfflineIcon } from './Icons'

export interface OfflineBannerProps {
  pendingCount?: number
}

export const OfflineBanner: React.FC<OfflineBannerProps> = ({ pendingCount = 0 }) => {
  const { t } = useTranslation()

  return (
    <aside
      className="offline-banner"
      role="status"
      aria-live="polite"
      data-testid="offline-banner"
    >
      <div className="offline-banner__content">
        <OfflineIcon />
        <span>{t('status.offlineBanner')}</span>
      </div>
      {pendingCount > 0 && (
        <span className="badge" style={{ fontVariantNumeric: 'tabular-nums' }}>
          {t('status.pendingChanges', { count: pendingCount })}
        </span>
      )}
    </aside>
  )
}
