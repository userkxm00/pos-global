// Authoritative Tauri IPC Client for Roles & Permissions Administration
// F1.16 — Roles / Permissions Administration UI
// Interfaces with local user and permission operations while preserving Rust authority

import type { User, CreateUserInput, UpdateUserInput } from '../types/user'
import type { Permission, Role, PermissionEffect, UserPermissionOverride } from '../types/permission'

export interface PermissionApiClient {
  listUsers(branchId: string): Promise<User[]>
  getUser(userId: string): Promise<User>
  createUser(input: CreateUserInput): Promise<User>
  updateUser(userId: string, input: UpdateUserInput): Promise<User>
  listRolePermissions(role: Role): Promise<Permission[]>
  listUserPermissionOverrides(userId: string): Promise<UserPermissionOverride[]>
  setUserPermissionOverride(userId: string, permission: Permission, effect: PermissionEffect): Promise<void>
  removeUserPermissionOverride(userId: string, permission: Permission): Promise<void>
  getEffectiveUserPermissions(userId: string): Promise<Permission[]>
}

export function extractInvokeErrorMessage(err: unknown): string {
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message
  return String(err)
}

const FALLBACK_ROLE_PERMISSIONS: Record<Role, readonly Permission[]> = {
  admin: [
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
  ],
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
}

function computeFallbackEffective(role: string, overrides: UserPermissionOverride[]): Permission[] {
  const normalized = role.trim().toLowerCase() as Role
  const defaults = FALLBACK_ROLE_PERMISSIONS[normalized] || []
  const set = new Set<Permission>(defaults)
  for (const o of overrides) {
    if (o.effect === 'deny') {
      set.delete(o.permission)
    } else if (o.effect === 'allow') {
      set.add(o.permission)
    }
  }
  return FALLBACK_ROLE_PERMISSIONS.admin.filter((p) => set.has(p))
}

// Real Tauri IPC Implementation
export class TauriPermissionApiClient implements PermissionApiClient {
  private async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      return await invoke<T>(cmd, args)
    } catch (err) {
      throw new Error(extractInvokeErrorMessage(err))
    }
  }

  async listUsers(branchId: string): Promise<User[]> {
    return this.invoke<User[]>('list_users', { branchId })
  }

  async getUser(userId: string): Promise<User> {
    return this.invoke<User>('get_user', { id: userId })
  }

  async createUser(input: CreateUserInput): Promise<User> {
    return this.invoke<User>('create_user', { input })
  }

  async updateUser(userId: string, input: UpdateUserInput): Promise<User> {
    return this.invoke<User>('update_user', { id: userId, input })
  }

  async listRolePermissions(role: Role): Promise<Permission[]> {
    try {
      return await this.invoke<Permission[]>('list_role_permissions', { role })
    } catch {
      return [...(FALLBACK_ROLE_PERMISSIONS[role] || [])]
    }
  }

  async listUserPermissionOverrides(userId: string): Promise<UserPermissionOverride[]> {
    return this.invoke<UserPermissionOverride[]>('list_user_permission_overrides', { userId })
  }

  async setUserPermissionOverride(
    userId: string,
    permission: Permission,
    effect: PermissionEffect,
  ): Promise<void> {
    return this.invoke<void>('set_user_permission_override', { userId, permission, effect })
  }

  async removeUserPermissionOverride(userId: string, permission: Permission): Promise<void> {
    return this.invoke<void>('remove_user_permission_override', { userId, permission })
  }

  async getEffectiveUserPermissions(userId: string): Promise<Permission[]> {
    try {
      return await this.invoke<Permission[]>('get_effective_user_permissions', { userId })
    } catch {
      const user = await this.getUser(userId)
      const overrides = await this.listUserPermissionOverrides(userId)
      return computeFallbackEffective(user.role, overrides)
    }
  }
}

// In-Memory Mock Implementation for Testing and Dev
export class MockPermissionApiClient implements PermissionApiClient {
  public users: User[] = []
  public overrides: Map<string, UserPermissionOverride[]> = new Map()
  public shouldFailWith: string | null = null
  public delayMs: number = 0

  constructor(initialUsers: User[] = []) {
    this.users = [...initialUsers]
  }

  private async maybeDelay(): Promise<void> {
    if (this.delayMs > 0) {
      await new Promise((resolve) => setTimeout(resolve, this.delayMs))
    }
  }

  async listUsers(branchId: string): Promise<User[]> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return this.users.filter((u) => u.branch_id === branchId)
  }

  async getUser(userId: string): Promise<User> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    const found = this.users.find((u) => u.id === userId)
    if (!found) throw new Error(`User '${userId}' not found`)
    return { ...found }
  }

  async createUser(input: CreateUserInput): Promise<User> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    if (!input.full_name || input.full_name.trim().length === 0) {
      throw new Error('User full name cannot be empty')
    }
    if (input.full_name.length > 255) {
      throw new Error('User full name cannot exceed 255 characters')
    }
    if (!input.role || input.role.trim().length === 0) {
      throw new Error('User role cannot be empty')
    }

    if (input.username && input.username.trim().length > 0) {
      const exists = this.users.some((u) => u.username === input.username?.trim())
      if (exists) {
        throw new Error(`Username '${input.username}' already exists`)
      }
    }

    const newUser: User = {
      id: `usr_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      branch_id: input.branch_id,
      full_name: input.full_name.trim(),
      username: input.username?.trim() || null,
      role: input.role.trim().toLowerCase(),
      is_active: true,
      supabase_user_id: input.supabase_user_id || null,
      auth_provider: input.auth_provider || 'local',
      created_at: new Date().toISOString(),
    }
    this.users.push(newUser)
    return { ...newUser }
  }

  async updateUser(userId: string, input: UpdateUserInput): Promise<User> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    const idx = this.users.findIndex((u) => u.id === userId)
    if (idx === -1) throw new Error(`User '${userId}' not found`)

    if (input.full_name !== undefined) {
      if (!input.full_name || input.full_name.trim().length === 0) {
        throw new Error('Full name cannot be empty')
      }
      if (input.full_name.length > 255) {
        throw new Error('Full name cannot exceed 255 characters')
      }
    }

    if (input.role !== undefined && (!input.role || input.role.trim().length === 0)) {
      throw new Error('Role cannot be empty')
    }

    if (input.username && input.username.trim().length > 0) {
      const exists = this.users.some((u) => u.username === input.username?.trim() && u.id !== userId)
      if (exists) {
        throw new Error(`Username '${input.username}' already exists`)
      }
    }

    const current = this.users[idx]
    const updated: User = {
      ...current,
      full_name: input.full_name !== undefined ? input.full_name.trim() : current.full_name,
      username: input.username !== undefined ? input.username?.trim() || null : current.username,
      role: input.role !== undefined ? input.role.trim().toLowerCase() : current.role,
      is_active: input.is_active !== undefined && input.is_active !== null ? input.is_active : current.is_active,
      supabase_user_id: input.supabase_user_id !== undefined ? input.supabase_user_id : current.supabase_user_id,
    }
    this.users[idx] = updated
    return { ...updated }
  }

  async listRolePermissions(role: Role): Promise<Permission[]> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return [...(FALLBACK_ROLE_PERMISSIONS[role] || [])]
  }

  async listUserPermissionOverrides(userId: string): Promise<UserPermissionOverride[]> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    return [...(this.overrides.get(userId) || [])]
  }

  async setUserPermissionOverride(
    userId: string,
    permission: Permission,
    effect: PermissionEffect,
  ): Promise<void> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    const existing = this.overrides.get(userId) || []
    const filtered = existing.filter((o) => o.permission !== permission)
    filtered.push({ permission, effect })
    this.overrides.set(userId, filtered)
  }

  async removeUserPermissionOverride(userId: string, permission: Permission): Promise<void> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    const existing = this.overrides.get(userId) || []
    const filtered = existing.filter((o) => o.permission !== permission)
    this.overrides.set(userId, filtered)
  }

  async getEffectiveUserPermissions(userId: string): Promise<Permission[]> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    const user = await this.getUser(userId)
    const userOverrides = await this.listUserPermissionOverrides(userId)
    return computeFallbackEffective(user.role, userOverrides)
  }
}

// Active client singleton instance
let activePermissionClient: PermissionApiClient = new TauriPermissionApiClient()

export function getPermissionApi(): PermissionApiClient {
  return activePermissionClient
}

export function setPermissionApi(client: PermissionApiClient): void {
  activePermissionClient = client
}
