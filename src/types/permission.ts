// Roles and Permissions domain types.
// F1.06 — Roles and Permissions & F1.16 — Roles / Permissions Administration UI

export type Permission =
  | 'sales.create'
  | 'sales.refund'
  | 'sales.void'
  | 'inventory.adjust'
  | 'inventory.transfer'
  | 'products.manage'
  | 'purchases.manage'
  | 'customers.manage'
  | 'debts.manage'
  | 'cash.open'
  | 'cash.close'
  | 'cash.adjust'
  | 'reports.view'
  | 'reports.export'
  | 'users.manage'
  | 'settings.manage'
  | 'license.manage'

export type Role = 'admin' | 'manager' | 'cashier'

export type PermissionEffect = 'allow' | 'deny'

export interface UserPermissionOverride {
  permission: Permission
  effect: PermissionEffect
}

export interface RolePermissionMapping {
  role: Role
  permissions: Permission[]
}

export type PermissionCategory =
  | 'sales'
  | 'inventory'
  | 'purchases'
  | 'customers'
  | 'cash'
  | 'reports'
  | 'administration'

export interface PermissionCatalogEntry {
  code: Permission
  category: PermissionCategory
  titleKey: string
  descriptionKey: string
}

export interface RoleCatalogEntry {
  role: Role
  titleKey: string
  descriptionKey: string
  defaultPermissions: Permission[]
}
