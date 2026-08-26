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

export const PERMISSION_CATALOG: readonly PermissionCatalogEntry[] = [
  // Sales
  {
    code: 'sales.create',
    category: 'sales',
    titleKey: 'permissions.items.salesCreate.title',
    descriptionKey: 'permissions.items.salesCreate.description',
  },
  {
    code: 'sales.refund',
    category: 'sales',
    titleKey: 'permissions.items.salesRefund.title',
    descriptionKey: 'permissions.items.salesRefund.description',
  },
  {
    code: 'sales.void',
    category: 'sales',
    titleKey: 'permissions.items.salesVoid.title',
    descriptionKey: 'permissions.items.salesVoid.description',
  },
  // Inventory
  {
    code: 'inventory.adjust',
    category: 'inventory',
    titleKey: 'permissions.items.inventoryAdjust.title',
    descriptionKey: 'permissions.items.inventoryAdjust.description',
  },
  {
    code: 'inventory.transfer',
    category: 'inventory',
    titleKey: 'permissions.items.inventoryTransfer.title',
    descriptionKey: 'permissions.items.inventoryTransfer.description',
  },
  {
    code: 'products.manage',
    category: 'inventory',
    titleKey: 'permissions.items.productsManage.title',
    descriptionKey: 'permissions.items.productsManage.description',
  },
  // Purchases
  {
    code: 'purchases.manage',
    category: 'purchases',
    titleKey: 'permissions.items.purchasesManage.title',
    descriptionKey: 'permissions.items.purchasesManage.description',
  },
  // Customers & Debts
  {
    code: 'customers.manage',
    category: 'customers',
    titleKey: 'permissions.items.customersManage.title',
    descriptionKey: 'permissions.items.customersManage.description',
  },
  {
    code: 'debts.manage',
    category: 'customers',
    titleKey: 'permissions.items.debtsManage.title',
    descriptionKey: 'permissions.items.debtsManage.description',
  },
  // Cash Management
  {
    code: 'cash.open',
    category: 'cash',
    titleKey: 'permissions.items.cashOpen.title',
    descriptionKey: 'permissions.items.cashOpen.description',
  },
  {
    code: 'cash.close',
    category: 'cash',
    titleKey: 'permissions.items.cashClose.title',
    descriptionKey: 'permissions.items.cashClose.description',
  },
  {
    code: 'cash.adjust',
    category: 'cash',
    titleKey: 'permissions.items.cashAdjust.title',
    descriptionKey: 'permissions.items.cashAdjust.description',
  },
  // Reports
  {
    code: 'reports.view',
    category: 'reports',
    titleKey: 'permissions.items.reportsView.title',
    descriptionKey: 'permissions.items.reportsView.description',
  },
  {
    code: 'reports.export',
    category: 'reports',
    titleKey: 'permissions.items.reportsExport.title',
    descriptionKey: 'permissions.items.reportsExport.description',
  },
  // Administration
  {
    code: 'users.manage',
    category: 'administration',
    titleKey: 'permissions.items.usersManage.title',
    descriptionKey: 'permissions.items.usersManage.description',
  },
  {
    code: 'settings.manage',
    category: 'administration',
    titleKey: 'permissions.items.settingsManage.title',
    descriptionKey: 'permissions.items.settingsManage.description',
  },
  {
    code: 'license.manage',
    category: 'administration',
    titleKey: 'permissions.items.licenseManage.title',
    descriptionKey: 'permissions.items.licenseManage.description',
  },
] as const

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
 * Deny overrides take precedence over role defaults and allow overrides.
 */
export function computeEffectivePermissions(
  roleStr: string,
  overrides: UserPermissionOverride[] = [],
): Permission[] {
  const defaults = getRoleDefaultPermissions(roleStr)
  const effective = new Set<Permission>(defaults)

  // Apply explicit overrides
  for (const override of overrides) {
    if (override.effect === 'deny') {
      effective.delete(override.permission)
    } else if (override.effect === 'allow') {
      if (AUTHORITATIVE_PERMISSIONS.includes(override.permission)) {
        effective.add(override.permission)
      }
    }
  }

  return AUTHORITATIVE_PERMISSIONS.filter((p) => effective.has(p))
}

/**
 * Deterministic permission check helper for presentation gating.
 * Backend Rust middleware remains authoritative for actual transaction authorization.
 */
export function hasEffectivePermission(
  roleStr: string,
  overrides: UserPermissionOverride[] = [],
  requiredPermission: Permission,
): boolean {
  const effective = computeEffectivePermissions(roleStr, overrides)
  return effective.includes(requiredPermission)
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
