// Permission Denied View Component
// F1.18 — Authorization and Error-State UX & UI_SPEC.md

import React from 'react'
import { useTranslation } from 'react-i18next'
import { LockIcon } from './Icons'
import type { Permission } from '../../types/permission'

export interface PermissionDeniedStateProps {
  permission?: Permission | null
  requiredRole?: string | null
  title?: string
  description?: string
  actionLabel?: string
  onAction?: () => void
}

export const PermissionDeniedState: React.FC<PermissionDeniedStateProps> = ({
  permission,
  requiredRole,
  title,
  description,
  actionLabel,
  onAction,
}) => {
  const { t } = useTranslation()

  return (
    <div
      className="state-container state-container--denied"
      role="alert"
      data-testid="permission-denied-state"
    >
      <div className="state-container__icon">
        <LockIcon size={28} />
      </div>
      <h2 className="state-container__title">
        {title || t('states.permissionDenied.title')}
      </h2>
      <p className="state-container__description">
        {description ||
          t('states.permissionDenied.description', {
            permission: permission || 'unspecified.action',
          })}
      </p>

      {requiredRole && (
        <div className="state-container__code" data-testid="permission-required-role">
          <small>{t('states.permissionDenied.requiredRole')}: <code>{requiredRole}</code></small>
        </div>
      )}

      {onAction && (
        <div style={{ marginBlockStart: 'var(--space-4)' }}>
          <button type="button" className="btn btn--primary" onClick={onAction} data-testid="permission-action-btn">
            {actionLabel || t('states.permissionDenied.action')}
          </button>
        </div>
      )}
    </div>
  )
}
