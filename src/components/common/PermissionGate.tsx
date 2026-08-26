// Declarative Permission Gate Component
// F1.18 — Authorization and Error-State UX & UI_SPEC.md

import React, { useMemo } from 'react'
import { useAuth } from '../../context/AuthContext'
import type { Permission, UserPermissionOverride } from '../../types/permission'
import { computeEffectivePermissions } from '../../context/permissionEvaluation'
import { PermissionDeniedState } from './PermissionDeniedState'

export interface PermissionGateProps {
  permission: Permission | Permission[]
  requireAll?: boolean
  fallback?: React.ReactNode
  showDeniedState?: boolean
  overrides?: UserPermissionOverride[]
  children: React.ReactNode
}

export function checkPermissions(
  role: string | undefined | null,
  required: Permission | Permission[],
  requireAll = false,
  overrides: UserPermissionOverride[] = [],
): boolean {
  if (!role) return false

  const effective = computeEffectivePermissions(role, overrides)
  const reqList = Array.isArray(required) ? required : [required]

  if (reqList.length === 0) return true

  if (requireAll) {
    return reqList.every((p) => effective.includes(p))
  }

  return reqList.some((p) => effective.includes(p))
}

export const PermissionGate: React.FC<PermissionGateProps> = ({
  permission,
  requireAll = false,
  fallback = null,
  showDeniedState = false,
  overrides = [],
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
