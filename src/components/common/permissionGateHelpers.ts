// Permission Gate Pure Evaluation Helpers
// F1.18 — Authorization and Error-State UX & UI_SPEC.md

import type { Permission, UserPermissionOverride } from '../../types/permission'
import { computeEffectivePermissions } from '../../context/permissionEvaluation'

export const EMPTY_OVERRIDES: readonly UserPermissionOverride[] = Object.freeze([])

export function checkPermissions(
  role: string | undefined | null,
  required: Permission | Permission[],
  requireAll = false,
  overrides: readonly UserPermissionOverride[] = EMPTY_OVERRIDES,
): boolean {
  if (!role) return false

  const effective = computeEffectivePermissions(role, overrides as UserPermissionOverride[])
  const reqList = Array.isArray(required) ? required : [required]

  if (reqList.length === 0) return true

  if (requireAll) {
    return reqList.every((p) => effective.includes(p))
  }

  return reqList.some((p) => effective.includes(p))
}
