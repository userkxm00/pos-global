import React from 'react'
import { useTranslation } from 'react-i18next'
import { AlertIcon } from './Icons'

export interface ErrorStateProps {
  title?: string
  message?: string | null
  onRetry?: () => void
  onReport?: () => void
  icon?: React.ReactNode
}

export const ErrorState: React.FC<ErrorStateProps> = ({
  title,
  message,
  onRetry,
  onReport,
  icon,
}) => {
  const { t } = useTranslation()

  return (
    <div
      className="state-container state-container--error"
      role="alert"
      data-testid="error-state"
    >
      <div className="state-container__icon">{icon || <AlertIcon />}</div>
      <h2 className="state-container__title">{title || t('states.error.title')}</h2>
      <p className="state-container__description">
        {message || t('states.error.description')}
      </p>
      <div style={{ display: 'flex', gap: 'var(--space-3)' }}>
        {onRetry && (
          <button type="button" className="btn btn--primary" onClick={onRetry}>
            {t('states.error.retry')}
          </button>
        )}
        {onReport && (
          <button type="button" className="btn btn--secondary" onClick={onReport}>
            {t('states.error.contactSupport')}
          </button>
        )}
      </div>
    </div>
  )
}
