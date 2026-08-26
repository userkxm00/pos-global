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

export type RolePermissionResolver = (role: Role | string) => Permission[]
export type EffectivePermissionResolver = (
  role: Role | string,
  overrides: UserPermissionOverride[],
) => Permission[]

let activeRolePermissionResolver: RolePermissionResolver | null = null
let activeEffectivePermissionResolver: EffectivePermissionResolver | null = null

/**
 * Injects domain permission resolvers to eliminate duplicated permission catalogs across modules.
 */
export function setPermissionResolvers(
  roleResolver: RolePermissionResolver | null,
  effectiveResolver: EffectivePermissionResolver | null,
): void {
  activeRolePermissionResolver = roleResolver
  activeEffectivePermissionResolver = effectiveResolver
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
      if (activeRolePermissionResolver) {
        return activeRolePermissionResolver(role)
      }
      return []
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
      if (activeEffectivePermissionResolver) {
        return activeEffectivePermissionResolver(user.role, overrides)
      }
      return []
    }
  }
}

let mockIdCounter = 0

function generateMockUserId(): string {
  mockIdCounter += 1
  return `usr_${Date.now()}_${mockIdCounter}`
}

function validateNameInput(name: string | null | undefined): void {
  if (name !== undefined) {
    if (!name || name.trim().length === 0) {
      throw new Error('Full name cannot be empty')
    }
    if (name.length > 255) {
      throw new Error('Full name cannot exceed 255 characters')
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

    const trimmedUsername = input.username?.trim()
    if (trimmedUsername && this.users.some((u) => u.username === trimmedUsername)) {
      throw new Error(`Username '${input.username}' already exists`)
    }

    const newUser: User = {
      id: generateMockUserId(),
      branch_id: input.branch_id,
      full_name: input.full_name.trim(),
      username: trimmedUsername ?? null,
      role: input.role.trim().toLowerCase(),
      is_active: true,
      supabase_user_id: input.supabase_user_id ?? null,
      auth_provider: input.auth_provider ?? 'local',
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

    validateNameInput(input.full_name)

    if (input.role !== undefined && (!input.role || input.role.trim().length === 0)) {
      throw new Error('Role cannot be empty')
    }

    const trimmedUsername = input.username?.trim()
    if (trimmedUsername && this.users.some((u) => u.username === trimmedUsername && u.id !== userId)) {
      throw new Error(`Username '${input.username}' already exists`)
    }

    const current = this.users[idx]
    const updated: User = {
      ...current,
      full_name: input.full_name !== undefined && input.full_name !== null ? input.full_name.trim() : current.full_name,
      username: input.username !== undefined ? trimmedUsername ?? null : current.username,
      role: input.role !== undefined && input.role !== null ? input.role.trim().toLowerCase() : current.role,
      is_active: input.is_active ?? current.is_active,
      supabase_user_id: input.supabase_user_id ?? current.supabase_user_id,
    }
    this.users[idx] = updated
    return { ...updated }
  }

  async listRolePermissions(role: Role): Promise<Permission[]> {
    await this.maybeDelay()
    if (this.shouldFailWith) throw new Error(this.shouldFailWith)
    if (activeRolePermissionResolver) {
      return activeRolePermissionResolver(role)
    }
    return []
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
    if (activeEffectivePermissionResolver) {
      return activeEffectivePermissionResolver(user.role, userOverrides)
    }
    return []
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
