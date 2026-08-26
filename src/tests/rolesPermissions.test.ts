// Deterministic Unit & Integration Tests for F1.16: Roles / Permissions Administration UI
// Tests catalog integrity, default role mappings, override precedence, API client operations, branch isolation, and i18n completeness.

import { describe, it, beforeEach } from 'node:test'
import assert from 'node:assert/strict'
import {
  AUTHORITATIVE_PERMISSIONS,
  AUTHORITATIVE_ROLES,
  PERMISSION_CATALOG,
  ROLE_CATALOG,
  ROLE_DEFAULT_PERMISSIONS,
  CATEGORY_ORDER,
  getRoleDefaultPermissions,
  computeEffectivePermissions,
  hasEffectivePermission,
  validateUserScope,
} from '../context/permissionEvaluation.ts'
import {
  MockPermissionApiClient,
  TauriPermissionApiClient,
  extractInvokeErrorMessage,
  getPermissionApi,
  setPermissionApi,
} from '../services/permissionApi.ts'
import { en, ar, fr, getDirectionForLocale } from '../i18n/index.ts'
import type { User, CreateUserInput } from '../types/user.ts'
import type { Permission, UserPermissionOverride } from '../types/permission.ts'

describe('F1.16 Roles & Permissions Administration Test Suite', () => {
  const sampleAdmin: User = {
    id: 'usr_admin_1',
    branch_id: 'branch_1',
    full_name: 'Alice Admin',
    username: 'admin_alice',
    role: 'admin',
    is_active: true,
    supabase_user_id: 'sub_123',
    auth_provider: 'local',
    created_at: '2026-08-25T00:00:00Z',
  }

  const sampleManager: User = {
    id: 'usr_mgr_1',
    branch_id: 'branch_1',
    full_name: 'Bob Manager',
    username: 'mgr_bob',
    role: 'manager',
    is_active: true,
    supabase_user_id: null,
    auth_provider: 'local',
    created_at: '2026-08-25T00:00:00Z',
  }

  const sampleCashier: User = {
    id: 'usr_csh_1',
    branch_id: 'branch_1',
    full_name: 'Charlie Cashier',
    username: 'csh_charlie',
    role: 'cashier',
    is_active: true,
    supabase_user_id: null,
    auth_provider: 'local',
    created_at: '2026-08-25T00:00:00Z',
  }

  const sampleOtherBranchUser: User = {
    id: 'usr_csh_2',
    branch_id: 'branch_2',
    full_name: 'Diana Branch2',
    username: 'diana_b2',
    role: 'cashier',
    is_active: true,
    supabase_user_id: null,
    auth_provider: 'local',
    created_at: '2026-08-25T00:00:00Z',
  }

  let mockApi: MockPermissionApiClient

  beforeEach(() => {
    mockApi = new MockPermissionApiClient([
      sampleAdmin,
      sampleManager,
      sampleCashier,
      sampleOtherBranchUser,
    ])
    setPermissionApi(mockApi)
  })

  // Test 1: Authoritative Catalog Integrity
  it('1. Authoritative permission catalog defines exactly 17 canonical permissions', () => {
    assert.strictEqual(AUTHORITATIVE_PERMISSIONS.length, 17)
    assert.strictEqual(PERMISSION_CATALOG.length, 17)
    assert.strictEqual(AUTHORITATIVE_ROLES.length, 3)
    assert.strictEqual(CATEGORY_ORDER.length, 7)

    // Verify all permissions in catalog are in authoritative array
    for (const entry of PERMISSION_CATALOG) {
      assert.strictEqual(AUTHORITATIVE_PERMISSIONS.includes(entry.code), true)
      assert.ok(entry.titleKey.length > 0)
      assert.ok(entry.descriptionKey.length > 0)
    }

    for (const roleEntry of ROLE_CATALOG) {
      assert.strictEqual(AUTHORITATIVE_ROLES.includes(roleEntry.role), true)
      assert.ok(roleEntry.titleKey.length > 0)
      assert.ok(roleEntry.descriptionKey.length > 0)
    }
  })

  // Test 2: Role Default Permission Mappings (F1.06 Conformance)
  it('2. Role default permissions match authoritative F1.06 domain catalog', () => {
    // Admin has all 17 permissions
    const adminPerms = getRoleDefaultPermissions('admin')
    assert.strictEqual(adminPerms.length, 17)
    assert.strictEqual(adminPerms.includes('users.manage'), true)
    assert.strictEqual(adminPerms.includes('license.manage'), true)

    // Manager has 15 permissions (all except users.manage and license.manage)
    const mgrPerms = getRoleDefaultPermissions('manager')
    assert.strictEqual(mgrPerms.length, 15)
    assert.strictEqual(mgrPerms.includes('sales.create'), true)
    assert.strictEqual(mgrPerms.includes('inventory.adjust'), true)
    assert.strictEqual(mgrPerms.includes('settings.manage'), true)
    assert.strictEqual(mgrPerms.includes('users.manage'), false)
    assert.strictEqual(mgrPerms.includes('license.manage'), false)

    // Cashier has 5 permissions
    const cshPerms = getRoleDefaultPermissions('cashier')
    assert.strictEqual(cshPerms.length, 5)
    assert.strictEqual(cshPerms.includes('sales.create'), true)
    assert.strictEqual(cshPerms.includes('customers.manage'), true)
    assert.strictEqual(cshPerms.includes('reports.view'), true)
    assert.strictEqual(cshPerms.includes('cash.open'), true)
    assert.strictEqual(cshPerms.includes('cash.close'), true)
    assert.strictEqual(cshPerms.includes('inventory.adjust'), false)
    assert.strictEqual(cshPerms.includes('users.manage'), false)

    // Unknown role returns empty
    assert.deepStrictEqual(getRoleDefaultPermissions('unknown_role'), [])
  })

  // Test 3: Effective Permission Computation & Deny Precedence
  it('3. computeEffectivePermissions correctly applies overrides with deny precedence', () => {
    // Base cashier has 5 permissions
    const baseCashierPerms = computeEffectivePermissions('cashier')
    assert.strictEqual(baseCashierPerms.length, 5)

    // Explicit allow adds a permission
    const allowOverride: UserPermissionOverride = {
      permission: 'inventory.adjust',
      effect: 'allow',
    }
    const withAllow = computeEffectivePermissions('cashier', [allowOverride])
    assert.strictEqual(withAllow.length, 6)
    assert.strictEqual(withAllow.includes('inventory.adjust'), true)

    // Explicit deny removes a default permission
    const denyOverride: UserPermissionOverride = {
      permission: 'sales.create',
      effect: 'deny',
    }
    const withDeny = computeEffectivePermissions('cashier', [denyOverride])
    assert.strictEqual(withDeny.length, 4)
    assert.strictEqual(withDeny.includes('sales.create'), false)

    // Deny takes precedence if both exist for same permission
    const contradictory: UserPermissionOverride[] = [
      { permission: 'sales.create', effect: 'allow' },
      { permission: 'sales.create', effect: 'deny' },
    ]
    const withContradictory = computeEffectivePermissions('cashier', contradictory)
    assert.strictEqual(withContradictory.includes('sales.create'), false)
  })

  // Test 4: Presentation Gating Helper
  it('4. hasEffectivePermission accurately gates UI capabilities', () => {
    // Admin has users.manage
    assert.strictEqual(hasEffectivePermission('admin', [], 'users.manage'), true)

    // Default cashier lacks users.manage
    assert.strictEqual(hasEffectivePermission('cashier', [], 'users.manage'), false)

    // Cashier with custom allow override has users.manage
    const cashierGranted: UserPermissionOverride[] = [{ permission: 'users.manage', effect: 'allow' }]
    assert.strictEqual(hasEffectivePermission('cashier', cashierGranted, 'users.manage'), true)

    // Admin with custom deny override lacks users.manage
    const adminDenied: UserPermissionOverride[] = [{ permission: 'users.manage', effect: 'deny' }]
    assert.strictEqual(hasEffectivePermission('admin', adminDenied, 'users.manage'), false)
  })

  // Test 5: Branch Scoping & User Listing Isolation
  it('5. listUsers returns only users belonging to the requested branch', async () => {
    const branch1Users = await mockApi.listUsers('branch_1')
    assert.strictEqual(branch1Users.length, 3)
    assert.strictEqual(branch1Users.every((u) => u.branch_id === 'branch_1'), true)

    const branch2Users = await mockApi.listUsers('branch_2')
    assert.strictEqual(branch2Users.length, 1)
    assert.strictEqual(branch2Users[0].id, 'usr_csh_2')

    // Empty branch returns empty array
    const emptyBranchUsers = await mockApi.listUsers('non_existent_branch')
    assert.strictEqual(emptyBranchUsers.length, 0)
  })

  // Test 6: User Scope Validation
  it('6. validateUserScope strictly enforces branch matching', () => {
    assert.strictEqual(validateUserScope(sampleAdmin, 'branch_1'), true)
    assert.strictEqual(validateUserScope(sampleAdmin, 'branch_2'), false)
    assert.strictEqual(validateUserScope(null, 'branch_1'), false)
    assert.strictEqual(validateUserScope(sampleAdmin, ''), false)
  })

  // Test 7: Create User Validation & Execution
  it('7. createUser validates input constraints and creates user record', async () => {
    const validInput: CreateUserInput = {
      branch_id: 'branch_1',
      full_name: 'David Assistant',
      username: 'david_asst',
      role: 'cashier',
      pin: '1234',
    }

    const created = await mockApi.createUser(validInput)
    assert.ok(created.id.startsWith('usr_'))
    assert.strictEqual(created.full_name, 'David Assistant')
    assert.strictEqual(created.username, 'david_asst')
    assert.strictEqual(created.role, 'cashier')
    assert.strictEqual(created.is_active, true)

    // Verify user is now listed in branch_1
    const updatedUsers = await mockApi.listUsers('branch_1')
    assert.strictEqual(updatedUsers.length, 4)

    // Empty name rejected
    await assert.rejects(
      () => mockApi.createUser({ ...validInput, full_name: '   ' }),
      /User full name cannot be empty/,
    )

    // Name exceeding 255 chars rejected
    await assert.rejects(
      () => mockApi.createUser({ ...validInput, full_name: 'A'.repeat(256) }),
      /User full name cannot exceed 255 characters/,
    )

    // Empty role rejected
    await assert.rejects(
      () => mockApi.createUser({ ...validInput, role: '' }),
      /User role cannot be empty/,
    )

    // Duplicate username rejected
    await assert.rejects(
      () => mockApi.createUser({ ...validInput, username: 'admin_alice' }),
      /Username 'admin_alice' already exists/,
    )
  })

  // Test 8: Update User & Role Reassignment
  it('8. updateUser updates role and status with validation', async () => {
    // Reassign Charlie from cashier to manager
    const updated = await mockApi.updateUser('usr_csh_1', { role: 'manager' })
    assert.strictEqual(updated.role, 'manager')

    const fetched = await mockApi.getUser('usr_csh_1')
    assert.strictEqual(fetched.role, 'manager')

    // Deactivate user
    const deactivated = await mockApi.updateUser('usr_csh_1', { is_active: false })
    assert.strictEqual(deactivated.is_active, false)

    // Empty name in update rejected
    await assert.rejects(
      () => mockApi.updateUser('usr_csh_1', { full_name: '' }),
      /Full name cannot be empty/,
    )

    // Name exceeding 255 chars rejected
    await assert.rejects(
      () => mockApi.updateUser('usr_csh_1', { full_name: 'B'.repeat(256) }),
      /Full name cannot exceed 255 characters/,
    )

    // Empty role in update rejected
    await assert.rejects(
      () => mockApi.updateUser('usr_csh_1', { role: '' }),
      /Role cannot be empty/,
    )

    // Duplicate username in update rejected
    await assert.rejects(
      () => mockApi.updateUser('usr_csh_1', { username: 'admin_alice' }),
      /Username 'admin_alice' already exists/,
    )

    // Non-existent user rejected
    await assert.rejects(
      () => mockApi.updateUser('non_existent', { role: 'manager' }),
      /User 'non_existent' not found/,
    )

    await assert.rejects(
      () => mockApi.getUser('non_existent'),
      /User 'non_existent' not found/,
    )
  })

  // Test 9: Permission Overrides Management API
  it('9. setUserPermissionOverride and removeUserPermissionOverride manage user overrides', async () => {
    // Initially Charlie has no overrides
    const initialOverrides = await mockApi.listUserPermissionOverrides('usr_csh_1')
    assert.strictEqual(initialOverrides.length, 0)

    // Add allow override for inventory.adjust
    await mockApi.setUserPermissionOverride('usr_csh_1', 'inventory.adjust', 'allow')
    const overrides1 = await mockApi.listUserPermissionOverrides('usr_csh_1')
    assert.strictEqual(overrides1.length, 1)
    assert.strictEqual(overrides1[0].permission, 'inventory.adjust')
    assert.strictEqual(overrides1[0].effect, 'allow')

    // Effective permissions reflects the override
    const effective1 = await mockApi.getEffectiveUserPermissions('usr_csh_1')
    assert.strictEqual(effective1.includes('inventory.adjust'), true)

    // Remove override
    await mockApi.removeUserPermissionOverride('usr_csh_1', 'inventory.adjust')
    const overrides2 = await mockApi.listUserPermissionOverrides('usr_csh_1')
    assert.strictEqual(overrides2.length, 0)

    const effective2 = await mockApi.getEffectiveUserPermissions('usr_csh_1')
    assert.strictEqual(effective2.includes('inventory.adjust'), false)
  })

  // Test 10: Mock Error Propagation and Delays
  it('10. MockPermissionApiClient propagates errors and delays deterministically', async () => {
    mockApi.delayMs = 2
    mockApi.shouldFailWith = 'Service failure simulation'

    await assert.rejects(() => mockApi.listUsers('branch_1'), /Service failure simulation/)
    await assert.rejects(() => mockApi.getUser('usr_admin_1'), /Service failure simulation/)
    await assert.rejects(() => mockApi.createUser({ branch_id: 'b1', full_name: 'Test', role: 'admin' }), /Service failure simulation/)
    await assert.rejects(() => mockApi.updateUser('usr_admin_1', { role: 'cashier' }), /Service failure simulation/)
    await assert.rejects(() => mockApi.listRolePermissions('admin'), /Service failure simulation/)
    await assert.rejects(() => mockApi.listUserPermissionOverrides('usr_admin_1'), /Service failure simulation/)
    await assert.rejects(() => mockApi.setUserPermissionOverride('usr_admin_1', 'sales.create', 'deny'), /Service failure simulation/)
    await assert.rejects(() => mockApi.removeUserPermissionOverride('usr_admin_1', 'sales.create'), /Service failure simulation/)
    await assert.rejects(() => mockApi.getEffectiveUserPermissions('usr_admin_1'), /Service failure simulation/)
  })

  // Test 11: TauriPermissionApiClient and Default Singleton
  it('11. TauriPermissionApiClient and singleton instance handle invocation environment gracefully', async () => {
    const tauriClient = new TauriPermissionApiClient()
    setPermissionApi(tauriClient)
    assert.strictEqual(getPermissionApi(), tauriClient)

    // listRolePermissions fallbacks gracefully outside Tauri
    const adminPerms = await tauriClient.listRolePermissions('admin')
    assert.strictEqual(adminPerms.length, 17)

    const cashierPerms = await tauriClient.listRolePermissions('cashier')
    assert.strictEqual(cashierPerms.length, 5)

    // Reset back to mockApi
    setPermissionApi(mockApi)
  })

  // Test 12: Error Message Extraction Helper
  it('12. extractInvokeErrorMessage extracts safe messages from various error shapes', () => {
    assert.strictEqual(extractInvokeErrorMessage('Plain string error'), 'Plain string error')
    assert.strictEqual(extractInvokeErrorMessage(new Error('Typed Error message')), 'Typed Error message')
    assert.strictEqual(extractInvokeErrorMessage({ custom: 123 }), '[object Object]')
  })

  // Test 13: i18n Completeness for Roles, Permissions, and Admin Keys
  it('13. i18n dictionaries provide complete translations in en, ar, and fr', () => {
    // Check all roles exist in all locales
    for (const role of AUTHORITATIVE_ROLES) {
      assert.ok(en.roles[role]?.title, `en missing title for role ${role}`)
      assert.ok(ar.roles[role]?.title, `ar missing title for role ${role}`)
      assert.ok(fr.roles[role]?.title, `fr missing title for role ${role}`)
    }

    // Check admin tabs and headers exist in all locales
    assert.ok(en.admin.tabs.users)
    assert.ok(ar.admin.tabs.users)
    assert.ok(fr.admin.tabs.users)

    assert.ok(en.admin.matrix.title)
    assert.ok(ar.admin.matrix.title)
    assert.ok(fr.admin.matrix.title)

    // Check RTL direction helper
    assert.strictEqual(getDirectionForLocale('ar'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar-DZ'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar_EG'), 'rtl')
    assert.strictEqual(getDirectionForLocale('en'), 'ltr')
    assert.strictEqual(getDirectionForLocale('fr'), 'ltr')
  })
})
