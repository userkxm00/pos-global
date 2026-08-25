import React, { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

export interface LoadingSkeletonProps {
  cardsCount?: number
  ariaLabel?: string
}

export const LoadingSkeleton: React.FC<LoadingSkeletonProps> = ({
  cardsCount = 6,
  ariaLabel,
}) => {
  const { t } = useTranslation()
  const label = ariaLabel || t('states.loading.title')
  const skeletonCards = useMemo(
    () => Array.from({ length: cardsCount }, (_, i) => `skeleton-item-card-${i + 1}`),
    [cardsCount],
  )

  return (
    <div
      className="state-container"
      aria-busy="true"
      aria-live="polite"
      data-testid="loading-skeleton"
    >
      <div style={{ width: '100%', display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
        <div className="skeleton-box" style={{ height: '32px', width: '240px' }} />
        <div className="skeleton-box" style={{ height: '18px', width: '380px' }} />
        <div className="skeleton-grid" style={{ marginBlockStart: 'var(--space-4)' }}>
          {skeletonCards.map((cardId) => (
            <div key={cardId} className="skeleton-box skeleton-card" />
          ))}
        </div>
      </div>
      <span className="sr-only">{label}</span>
    </div>
  )
}
