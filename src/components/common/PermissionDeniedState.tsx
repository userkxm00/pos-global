import React from 'react'
import { useTranslation } from 'react-i18next'
import { LockIcon } from './Icons'
import type { Permission } from '../../types/permission'

export interface PermissionDeniedStateProps {
  permission?: Permission | null
  title?: string
  description?: string
  onAction?: () => void
}

export const PermissionDeniedState: React.FC<PermissionDeniedStateProps> = ({
  permission,
  title,
  description,
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
      {onAction && (
        <button type="button" className="btn btn--primary" onClick={onAction}>
          {t('states.permissionDenied.action')}
        </button>
      )}
    </div>
  )
}
