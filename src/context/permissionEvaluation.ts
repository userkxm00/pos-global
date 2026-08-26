// Authoritative Domain Evaluation Helpers and Catalogs
// F1.06 — Roles and Permissions & F1.16 — Roles / Permissions Administration UI

import type {
  Permission,
  Role,
  PermissionCategory,
  PermissionCatalogEntry,
  RoleCatalogEntry,
  UserPermissionOverride,
} from '../types/permission'

export const AUTHORITATIVE_PERMISSIONS: readonly Permission[] = [
  'sales.create',
  'sales.refund',
  'sales.void',
  'inventory.adjust',
  'inventory.transfer',
  'products.manage',
  'purchases.manage',
  'customers.manage',
  'debts.manage',
  'cash.open',
  'cash.close',
  'cash.adjust',
  'reports.view',
  'reports.export',
  'users.manage',
  'settings.manage',
  'license.manage',
] as const

export const AUTHORITATIVE_ROLES: readonly Role[] = ['admin', 'manager', 'cashier'] as const

type RawPermDef = readonly [Permission, PermissionCategory, string]

const RAW_PERM_ENTRIES: readonly RawPermDef[] = [
  ['sales.create', 'sales', 'salesCreate'],
  ['sales.refund', 'sales', 'salesRefund'],
  ['sales.void', 'sales', 'salesVoid'],
  ['inventory.adjust', 'inventory', 'inventoryAdjust'],
  ['inventory.transfer', 'inventory', 'inventoryTransfer'],
  ['products.manage', 'inventory', 'productsManage'],
  ['purchases.manage', 'purchases', 'purchasesManage'],
  ['customers.manage', 'customers', 'customersManage'],
  ['debts.manage', 'customers', 'debtsManage'],
  ['cash.open', 'cash', 'cashOpen'],
  ['cash.close', 'cash', 'cashClose'],
  ['cash.adjust', 'cash', 'cashAdjust'],
  ['reports.view', 'reports', 'reportsView'],
  ['reports.export', 'reports', 'reportsExport'],
  ['users.manage', 'administration', 'usersManage'],
  ['settings.manage', 'administration', 'settingsManage'],
  ['license.manage', 'administration', 'licenseManage'],
] as const

export const PERMISSION_CATALOG: readonly PermissionCatalogEntry[] = RAW_PERM_ENTRIES.map(
  ([code, category, keyPrefix]) => ({
    code,
    category,
    titleKey: `permissions.items.${keyPrefix}.title`,
    descriptionKey: `permissions.items.${keyPrefix}.description`,
  }),
)

export const ROLE_DEFAULT_PERMISSIONS: Record<Role, readonly Permission[]> = {
  admin: AUTHORITATIVE_PERMISSIONS,
  manager: [
    'sales.create',
    'sales.refund',
    'sales.void',
    'inventory.adjust',
    'inventory.transfer',
    'products.manage',
    'purchases.manage',
    'customers.manage',
    'debts.manage',
    'cash.open',
    'cash.close',
    'cash.adjust',
    'reports.view',
    'reports.export',
    'settings.manage',
  ],
  cashier: [
    'sales.create',
    'customers.manage',
    'reports.view',
    'cash.open',
    'cash.close',
  ],
} as const

export const ROLE_CATALOG: readonly RoleCatalogEntry[] = [
  {
    role: 'admin',
    titleKey: 'roles.admin.title',
    descriptionKey: 'roles.admin.description',
    defaultPermissions: [...ROLE_DEFAULT_PERMISSIONS.admin],
  },
  {
    role: 'manager',
    titleKey: 'roles.manager.title',
    descriptionKey: 'roles.manager.description',
    defaultPermissions: [...ROLE_DEFAULT_PERMISSIONS.manager],
  },
  {
    role: 'cashier',
    titleKey: 'roles.cashier.title',
    descriptionKey: 'roles.cashier.description',
    defaultPermissions: [...ROLE_DEFAULT_PERMISSIONS.cashier],
  },
] as const

export const CATEGORY_ORDER: readonly PermissionCategory[] = [
  'sales',
  'inventory',
  'purchases',
  'customers',
  'cash',
  'reports',
  'administration',
] as const

/**
 * Returns default built-in permissions for a role matching F1.06 specification.
 */
export function getRoleDefaultPermissions(roleStr: string): Permission[] {
  const normalized = roleStr.trim().toLowerCase()
  if (normalized === 'admin') return [...ROLE_DEFAULT_PERMISSIONS.admin]
  if (normalized === 'manager') return [...ROLE_DEFAULT_PERMISSIONS.manager]
  if (normalized === 'cashier') return [...ROLE_DEFAULT_PERMISSIONS.cashier]
  return []
}

/**
 * Computes effective permissions for a user given their role and explicit overrides.
 * Deny overrides take strict precedence over allow overrides and role defaults,
 * regardless of array order.
 */
export function computeEffectivePermissions(
  roleStr: string,
  overrides: UserPermissionOverride[] = [],
): Permission[] {
  const defaults = getRoleDefaultPermissions(roleStr)
  const effective = new Set<Permission>(defaults)

  // 1. Apply allow overrides
  for (const override of overrides) {
    if (override.effect === 'allow' && AUTHORITATIVE_PERMISSIONS.includes(override.permission)) {
      effective.add(override.permission)
    }
  }

  // 2. Apply deny overrides (deny strictly wins)
  for (const override of overrides) {
    if (override.effect === 'deny') {
      effective.delete(override.permission)
    }
  }

  return AUTHORITATIVE_PERMISSIONS.filter((p) => effective.has(p))
}

export const EMPTY_OVERRIDES: readonly UserPermissionOverride[] = Object.freeze([])

/**
 * Deterministic permission check helper for presentation gating.
 * Backend Rust middleware remains authoritative for actual transaction authorization.
 */
export function hasEffectivePermission(
  roleStr: string,
  requiredPermission: Permission,
  overrides: readonly UserPermissionOverride[] = EMPTY_OVERRIDES,
): boolean {
  const effective = computeEffectivePermissions(roleStr, overrides as UserPermissionOverride[])
  return effective.includes(requiredPermission)
}

/**
 * Evaluates single or multi-permission requirements against active user role and overrides.
 * Fails closed for unauthenticated roles and empty permission requirement arrays.
 */
export function checkPermissions(
  role: string | undefined | null,
  required: Permission | Permission[],
  requireAll = false,
  overrides: readonly UserPermissionOverride[] = EMPTY_OVERRIDES,
): boolean {
  if (!role) return false

  const reqList = Array.isArray(required) ? required : [required]
  // Fail closed when permission requirements are empty or invalid
  if (reqList.length === 0) return false

  const effective = computeEffectivePermissions(role, overrides as UserPermissionOverride[])

  if (requireAll) {
    return reqList.every((p) => effective.includes(p))
  }

  return reqList.some((p) => effective.includes(p))
}

/**
 * Validates that a user record matches the active branch context.
 * Prevents cross-branch leakage.
 */
export function validateUserScope(
  user: { branch_id: string } | null | undefined,
  activeBranchId: string,
): boolean {
  if (!user || !activeBranchId) return false
  return user.branch_id === activeBranchId
}
