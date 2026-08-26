// Error State View Component
// F1.18 — Authorization and Error-State UX & UI_SPEC.md

import React from 'react'
import { useTranslation } from 'react-i18next'
import { AlertIcon } from './Icons'

export interface ErrorStateProps {
  title?: string
  message?: string | null
  errorCode?: string | null
  correlationId?: string | null
  retryLabel?: string
  reportLabel?: string
  onRetry?: () => void
  onReport?: () => void
  icon?: React.ReactNode
}

export const ErrorState: React.FC<ErrorStateProps> = ({
  title,
  message,
  errorCode,
  correlationId,
  retryLabel,
  reportLabel,
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

      {errorCode && (
        <div className="state-container__code" data-testid="error-code-badge">
          <code>{errorCode}</code>
        </div>
      )}

      {correlationId && (
        <div className="state-container__correlation" data-testid="error-correlation-badge">
          <small>ID: {correlationId}</small>
        </div>
      )}

      <div style={{ display: 'flex', gap: 'var(--space-3)', marginBlockStart: 'var(--space-4)' }}>
        {onRetry && (
          <button type="button" className="btn btn--primary" onClick={onRetry} data-testid="error-retry-btn">
            {retryLabel || t('states.error.retry')}
          </button>
        )}
        {onReport && (
          <button type="button" className="btn btn--secondary" onClick={onReport} data-testid="error-report-btn">
            {reportLabel || t('states.error.contactSupport')}
          </button>
        )}
      </div>
    </div>
  )
}
