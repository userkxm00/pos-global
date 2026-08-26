// Declarative Permission Gate Component
// F1.18 — Authorization and Error-State UX & UI_SPEC.md

import React, { useMemo } from 'react'
import { useAuth } from '../../context/AuthContext'
import type { Permission, UserPermissionOverride } from '../../types/permission'
import { PermissionDeniedState } from './PermissionDeniedState'
import { checkPermissions, EMPTY_OVERRIDES } from './permissionGateHelpers'

export { checkPermissions, EMPTY_OVERRIDES }

export interface PermissionGateProps {
  permission: Permission | Permission[]
  requireAll?: boolean
  fallback?: React.ReactNode
  showDeniedState?: boolean
  overrides?: readonly UserPermissionOverride[]
  children: React.ReactNode
}

export const PermissionGate: React.FC<PermissionGateProps> = ({
  permission,
  requireAll = false,
  fallback = null,
  showDeniedState = false,
  overrides = EMPTY_OVERRIDES,
  children,
}) => {
  const { activeUser, authStatus } = useAuth()

  const isAllowed = useMemo(() => {
    if (authStatus !== 'authenticated' || !activeUser) {
      return false
    }
    return checkPermissions(activeUser.role, permission, requireAll, overrides)
  }, [authStatus, activeUser, permission, requireAll, overrides])

  if (isAllowed) {
    return <>{children}</>
  }

  if (fallback !== null && fallback !== undefined) {
    return <>{fallback}</>
  }

  if (showDeniedState) {
    const displayPerm = Array.isArray(permission) ? permission[0] : permission
    return <PermissionDeniedState permission={displayPerm} />
  }

  return null
}
