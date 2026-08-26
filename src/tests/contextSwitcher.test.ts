// Deterministic Unit & Integration Tests for F1.15: Organization / Branch / Register Context Switcher
// Tests hierarchy invariants, cascading invalidation, atomic context switching, error handling, i18n, and accessibility.

import { describe, it, beforeEach } from 'node:test'
import assert from 'node:assert/strict'
import {
  validateContextHierarchy,
  isBranchCompatible,
  isRegisterCompatible,
  resolveBranchOnOrgChange,
  resolveRegisterOnBranchChange,
} from '../context/contextSwitching.ts'
import {
  MockContextApiClient,
  TauriContextApiClient,
  extractInvokeErrorMessage,
  getContextApi,
  setContextApi,
} from '../services/contextApi.ts'
import { en, ar, fr, getDirectionForLocale } from '../i18n/index.ts'
import type { Organization } from '../types/organization.ts'
import type { Branch } from '../types/branch.ts'
import type { Register } from '../types/register.ts'

describe('F1.15 Context Switcher Test Suite', () => {
  const orgA: Organization = {
    id: 'org_a',
    name: 'Acme Retail Corp',
    default_currency: 'USD',
    default_language: 'en',
    created_at: '2026-08-25T00:00:00Z',
  }

  const orgB: Organization = {
    id: 'org_b',
    name: 'Global Mart Ltd',
    default_currency: 'EUR',
    default_language: 'fr',
    created_at: '2026-08-25T00:00:00Z',
  }

  const branchA1: Branch = {
    id: 'branch_a1',
    organization_id: 'org_a',
    name: 'Downtown Flagship',
    address: '123 Main St',
    currency: 'USD',
    is_active: true,
    created_at: '2026-08-25T00:00:00Z',
  }

  const branchA2: Branch = {
    id: 'branch_a2',
    organization_id: 'org_a',
    name: 'Uptown Branch',
    address: '456 High St',
    currency: 'USD',
    is_active: true,
    created_at: '2026-08-25T00:00:00Z',
  }

  const branchB1: Branch = {
    id: 'branch_b1',
    organization_id: 'org_b',
    name: 'Paris Center',
    address: '10 Rue de la Paix',
    currency: 'EUR',
    is_active: true,
    created_at: '2026-08-25T00:00:00Z',
  }

  const registerA1_1: Register = {
    id: 'reg_a1_1',
    organization_id: 'org_a',
    branch_id: 'branch_a1',
    name: 'Checkout Counter 01',
    code: 'POS-01',
    is_active: true,
    created_at: '2026-08-25T00:00:00Z',
  }

  const registerA1_2: Register = {
    id: 'reg_a1_2',
    organization_id: 'org_a',
    branch_id: 'branch_a1',
    name: 'Express Lane 02',
    code: 'POS-02',
    is_active: true,
    created_at: '2026-08-25T00:00:00Z',
  }

  const registerB1_1: Register = {
    id: 'reg_b1_1',
    organization_id: 'org_b',
    branch_id: 'branch_b1',
    name: 'Caisse Principale',
    code: 'CP-01',
    is_active: true,
    created_at: '2026-08-25T00:00:00Z',
  }

  let mockApi: MockContextApiClient

  beforeEach(() => {
    mockApi = new MockContextApiClient()
    mockApi.organizations = [orgA, orgB]
    mockApi.branches = [branchA1, branchA2, branchB1]
    mockApi.registers = [registerA1_1, registerA1_2, registerB1_1]
    setContextApi(mockApi)
  })

  // Test 1: validateContextHierarchy
  it('1. validateContextHierarchy correctly enforces Organization -> Branch -> Register invariants', () => {
    // Valid hierarchy
    assert.strictEqual(validateContextHierarchy(orgA, branchA1, registerA1_1), true)
    assert.strictEqual(validateContextHierarchy(orgB, branchB1, registerB1_1), true)

    // Null/undefined values
    assert.strictEqual(validateContextHierarchy(null, branchA1, registerA1_1), false)
    assert.strictEqual(validateContextHierarchy(orgA, null, registerA1_1), false)
    assert.strictEqual(validateContextHierarchy(orgA, branchA1, null), false)
    assert.strictEqual(validateContextHierarchy(undefined, undefined, undefined), false)

    // Incompatible Branch with Org
    assert.strictEqual(validateContextHierarchy(orgA, branchB1, registerA1_1), false)

    // Incompatible Register with Branch
    assert.strictEqual(validateContextHierarchy(orgA, branchA2, registerA1_1), false)

    // Incompatible Register with Org (cross-tenant mismatch)
    const corruptedReg: Register = { ...registerA1_1, organization_id: 'org_b' }
    assert.strictEqual(validateContextHierarchy(orgA, branchA1, corruptedReg), false)
  })

  // Test 2: Compatibility Check Helpers
  it('2. isBranchCompatible and isRegisterCompatible validate structural parent-child relationships', () => {
    assert.strictEqual(isBranchCompatible(branchA1, 'org_a'), true)
    assert.strictEqual(isBranchCompatible(branchA1, 'org_b'), false)
    assert.strictEqual(isBranchCompatible(null, 'org_a'), false)
    assert.strictEqual(isBranchCompatible(branchA1, null), false)
    assert.strictEqual(isBranchCompatible(undefined, undefined), false)

    assert.strictEqual(isRegisterCompatible(registerA1_1, 'org_a', 'branch_a1'), true)
    assert.strictEqual(isRegisterCompatible(registerA1_1, 'org_a', 'branch_a2'), false)
    assert.strictEqual(isRegisterCompatible(registerA1_1, 'org_b', 'branch_a1'), false)
    assert.strictEqual(isRegisterCompatible(null, 'org_a', 'branch_a1'), false)
    assert.strictEqual(isRegisterCompatible(registerA1_1, null, 'branch_a1'), false)
    assert.strictEqual(isRegisterCompatible(registerA1_1, 'org_a', null), false)
  })

  // Test 3: Cascading Invalidation on Org Change
  it('3. resolveBranchOnOrgChange preserves valid branch or invalidates stale branch', () => {
    const branchesOfA = [branchA1, branchA2]
    // If switching within Org A and current branch is branchA1, preserve branchA1
    assert.deepStrictEqual(resolveBranchOnOrgChange('org_a', branchA1, branchesOfA), branchA1)

    // If switching to Org B, branchA1 does not belong to Org B -> returns null
    assert.strictEqual(resolveBranchOnOrgChange('org_b', branchA1, [branchB1]), null)

    // If Org ID is empty or branch is null -> returns null
    assert.strictEqual(resolveBranchOnOrgChange('', branchA1, branchesOfA), null)
    assert.strictEqual(resolveBranchOnOrgChange('org_a', null, branchesOfA), null)
    assert.strictEqual(resolveBranchOnOrgChange(undefined, branchA1, branchesOfA), null)
  })

  // Test 4: Cascading Invalidation on Branch Change
  it('4. resolveRegisterOnBranchChange preserves valid register or invalidates stale register', () => {
    const regsOfA1 = [registerA1_1, registerA1_2]
    // If staying on branchA1, registerA1_1 is valid -> preserved
    assert.deepStrictEqual(
      resolveRegisterOnBranchChange('org_a', 'branch_a1', registerA1_1, regsOfA1),
      registerA1_1,
    )

    // If switching to branchA2, registerA1_1 belongs to branch_a1 -> returns null
    assert.strictEqual(
      resolveRegisterOnBranchChange('org_a', 'branch_a2', registerA1_1, []),
      null,
    )

    // If switching to Org B branch B1 -> returns null
    assert.strictEqual(
      resolveRegisterOnBranchChange('org_b', 'branch_b1', registerA1_1, [registerB1_1]),
      null,
    )

    // If Org ID, Branch ID, or Register is empty -> returns null
    assert.strictEqual(resolveRegisterOnBranchChange('', 'branch_a1', registerA1_1, regsOfA1), null)
    assert.strictEqual(resolveRegisterOnBranchChange('org_a', '', registerA1_1, regsOfA1), null)
    assert.strictEqual(resolveRegisterOnBranchChange('org_a', 'branch_a1', null, regsOfA1), null)
  })

  // Test 5: ContextApiClient loading operations
  it('5. ContextApiClient loads organizations, branches, and registers with tenant isolation', async () => {
    const api = getContextApi()
    const orgs = await api.listOrganizations()
    assert.strictEqual(orgs.length, 2)
    assert.strictEqual(orgs[0].name, 'Acme Retail Corp')

    const branchesForA = await api.listBranches('org_a')
    assert.strictEqual(branchesForA.length, 2)
    assert.strictEqual(branchesForA[0].name, 'Downtown Flagship')

    const branchesForB = await api.listBranches('org_b')
    assert.strictEqual(branchesForB.length, 1)
    assert.strictEqual(branchesForB[0].name, 'Paris Center')

    const regsForA1 = await api.listRegisters('branch_a1')
    assert.strictEqual(regsForA1.length, 2)

    const regsForB1 = await api.listRegisters('branch_b1')
    assert.strictEqual(regsForB1.length, 1)
    assert.strictEqual(regsForB1[0].name, 'Caisse Principale')
  })

  // Test 6: Error Handling and Delay in ContextApiClient
  it('6. ContextApiClient propagates errors and delays deterministically when fetch fails', async () => {
    mockApi.delayMs = 1
    mockApi.shouldFailWith = 'Database query failed'
    const api = getContextApi()

    await assert.rejects(async () => {
      await api.listOrganizations()
    }, /Database query failed/)

    await assert.rejects(async () => {
      await api.listBranches('org_a')
    }, /Database query failed/)

    await assert.rejects(async () => {
      await api.listRegisters('branch_a1')
    }, /Database query failed/)
  })

  // Test 7: i18n Translation Parity across English, Arabic, and French
  it('7. i18n resources maintain complete parity for contextSwitcher keys', () => {
    const requiredKeys = [
      'title',
      'subtitle',
      'trigger',
      'triggerAriaLabel',
      'currentContext',
      'organizationLabel',
      'branchLabel',
      'registerLabel',
      'selectOrgPlaceholder',
      'selectBranchPlaceholder',
      'selectRegisterPlaceholder',
      'apply',
      'cancel',
      'close',
      'loading',
      'noOrganizations',
      'noBranches',
      'noRegisters',
      'retry',
      'success',
    ]

    for (const key of requiredKeys) {
      assert.ok((en.contextSwitcher as Record<string, unknown>)[key], `Missing en.contextSwitcher.${key}`)
      assert.ok((ar.contextSwitcher as Record<string, unknown>)[key], `Missing ar.contextSwitcher.${key}`)
      assert.ok((fr.contextSwitcher as Record<string, unknown>)[key], `Missing fr.contextSwitcher.${key}`)
    }

    assert.ok(en.contextSwitcher.errors.loadFailed)
    assert.ok(ar.contextSwitcher.errors.loadFailed)
    assert.ok(fr.contextSwitcher.errors.loadFailed)

    assert.ok(en.contextSwitcher.errors.unauthorized)
    assert.ok(ar.contextSwitcher.errors.unauthorized)
    assert.ok(fr.contextSwitcher.errors.unauthorized)

    assert.ok(en.contextSwitcher.errors.invalidHierarchy)
    assert.ok(ar.contextSwitcher.errors.invalidHierarchy)
    assert.ok(fr.contextSwitcher.errors.invalidHierarchy)
  })

  // Test 8: Arabic RTL Direction Verification
  it('8. getDirectionForLocale correctly returns rtl for Arabic and ltr for English and French', () => {
    assert.strictEqual(getDirectionForLocale('ar'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar-DZ'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar-SA'), 'rtl')
    assert.strictEqual(getDirectionForLocale('en'), 'ltr')
    assert.strictEqual(getDirectionForLocale('fr'), 'ltr')
    assert.strictEqual(getDirectionForLocale(''), 'ltr')
    assert.strictEqual(getDirectionForLocale(null), 'ltr')
  })

  // Test 9: extractInvokeErrorMessage Helper
  it('9. extractInvokeErrorMessage extracts messages correctly from strings, Errors, and unknown types', () => {
    assert.strictEqual(extractInvokeErrorMessage('Custom error string'), 'Custom error string')
    assert.strictEqual(extractInvokeErrorMessage(new Error('Standard JS Error')), 'Standard JS Error')
    assert.strictEqual(extractInvokeErrorMessage(404), '404')
    assert.strictEqual(extractInvokeErrorMessage({ code: 500 }), '[object Object]')
  })

  // Test 10: TauriContextApiClient Fallback Handling
  it('10. TauriContextApiClient handles invocation environment gracefully', async () => {
    const tauriClient = new TauriContextApiClient()
    // In pure Node.js test environment, @tauri-apps/api/core is not present so invoke throws cleanly
    await assert.rejects(async () => {
      await tauriClient.listOrganizations()
    })
    await assert.rejects(async () => {
      await tauriClient.listBranches('org_1')
    })
    await assert.rejects(async () => {
      await tauriClient.listRegisters('branch_1')
    })
  })
})
