import React from 'react'
import { useTranslation } from 'react-i18next'

export interface EmptyStateProps {
  title?: string
  description?: string
  actionLabel?: string
  onAction?: () => void
  icon?: React.ReactNode
}

export const EmptyState: React.FC<EmptyStateProps> = ({
  title,
  description,
  actionLabel,
  onAction,
  icon,
}) => {
  const { t } = useTranslation()

  const defaultIcon = (
    <svg
      width="28"
      height="28"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
      <line x1="8" y1="21" x2="16" y2="21" />
      <line x1="12" y1="17" x2="12" y2="21" />
    </svg>
  )

  return (
    <div className="state-container state-container--empty" data-testid="empty-state">
      <div className="state-container__icon">{icon || defaultIcon}</div>
      <h2 className="state-container__title">{title || t('states.empty.title')}</h2>
      <p className="state-container__description">{description || t('states.empty.description')}</p>
      {onAction && (
        <button type="button" className="btn btn--primary" onClick={onAction}>
          {actionLabel || t('states.empty.action')}
        </button>
      )}
    </div>
  )
}
