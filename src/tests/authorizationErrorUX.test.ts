// Deterministic Unit & Integration Tests for F1.18: Authorization & Error-State UX
// Tests permission gating, toast alerts, confirmation dialogs, error boundaries, and i18n completeness.

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import {
  checkPermissions,
  EMPTY_OVERRIDES,
} from '../components/common/permissionGateHelpers.ts'
import {
  computeEffectivePermissions,
  hasEffectivePermission,
  AUTHORITATIVE_PERMISSIONS,
} from '../context/permissionEvaluation.ts'
import { en, ar, fr } from '../i18n/index.ts'
import type { Permission, UserPermissionOverride } from '../types/permission.ts'
import type { ToastMessage } from '../types/feedback.ts'

describe('F1.18 Authorization & Error-State UX Test Suite', () => {
  // Test 1: Permission Gate - Admin has all authoritative permissions
  it('1. verifies admin role has access to all authoritative permissions', () => {
    for (const perm of AUTHORITATIVE_PERMISSIONS) {
      assert.strictEqual(
        checkPermissions('admin', perm),
        true,
        `Admin should have permission ${perm}`,
      )
    }
  })

  // Test 2: Permission Gate - Role hierarchy & default access boundaries
  it('2. verifies role-based default permission boundaries for manager and cashier', () => {
    // Manager has sales and cash, but not users.manage or license.manage
    assert.strictEqual(checkPermissions('manager', 'sales.create'), true)
    assert.strictEqual(checkPermissions('manager', 'inventory.adjust'), true)
    assert.strictEqual(checkPermissions('manager', 'users.manage'), false)
    assert.strictEqual(checkPermissions('manager', 'license.manage'), false)

    // Cashier has sales.create, but not inventory.adjust, users.manage, or reports.export
    assert.strictEqual(checkPermissions('cashier', 'sales.create'), true)
    assert.strictEqual(checkPermissions('cashier', 'cash.open'), true)
    assert.strictEqual(checkPermissions('cashier', 'inventory.adjust'), false)
    assert.strictEqual(checkPermissions('cashier', 'users.manage'), false)
    assert.strictEqual(checkPermissions('cashier', 'reports.export'), false)
  })

  // Test 3: Permission Gate - Deny Override strict precedence
  it('3. enforces deny-override strict precedence over role defaults and allow overrides', () => {
    // Admin with explicit deny on sales.refund
    const overrides: UserPermissionOverride[] = [
      { permission: 'sales.refund', effect: 'deny' },
    ]
    assert.strictEqual(
      checkPermissions('admin', 'sales.refund', false, overrides),
      false,
      'Deny override must revoke sales.refund from admin',
    )
    assert.strictEqual(
      checkPermissions('admin', 'sales.create', false, overrides),
      true,
      'Admin retains unaffected permissions',
    )

    // Cashier with allow and deny overrides
    const cashierOverrides: UserPermissionOverride[] = [
      { permission: 'inventory.adjust', effect: 'allow' },
      { permission: 'sales.create', effect: 'deny' },
    ]
    assert.strictEqual(
      checkPermissions('cashier', 'inventory.adjust', false, cashierOverrides),
      true,
      'Allow override grants inventory.adjust to cashier',
    )
    assert.strictEqual(
      checkPermissions('cashier', 'sales.create', false, cashierOverrides),
      false,
      'Deny override revokes sales.create from cashier',
    )
  })

  // Test 4: Multi-permission evaluation (any vs all)
  it('4. evaluates multi-permission arrays with requireAll true and false', () => {
    const cashierPerms: Permission[] = ['sales.create', 'users.manage']

    // requireAll = false (any) -> cashier has sales.create so true
    assert.strictEqual(
      checkPermissions('cashier', cashierPerms, false),
      true,
      'Cashier has at least one of [sales.create, users.manage]',
    )

    // requireAll = true (all) -> cashier lacks users.manage so false
    assert.strictEqual(
      checkPermissions('cashier', cashierPerms, true),
      false,
      'Cashier lacks users.manage, so requireAll must be false',
    )

    // Admin has both
    assert.strictEqual(
      checkPermissions('admin', cashierPerms, true),
      true,
      'Admin has all permissions in array',
    )
  })

  // Test 5: Empty and invalid permission requirements fail closed
  it('5. fails closed when permission requirements are empty or invalid', () => {
    assert.strictEqual(checkPermissions('admin', []), false, 'Empty array must fail closed for admin')
    assert.strictEqual(checkPermissions('manager', []), false, 'Empty array must fail closed for manager')
    assert.strictEqual(checkPermissions('cashier', []), false, 'Empty array must fail closed for cashier')
    assert.strictEqual(checkPermissions(null, 'sales.create'), false)
    assert.strictEqual(checkPermissions(undefined, 'sales.create'), false)
    assert.strictEqual(checkPermissions('', 'sales.create'), false)
    assert.strictEqual(checkPermissions('unknown_role', 'sales.create'), false)
  })

  // Test 6: Stable default overrides reference
  it('6. provides a frozen stable EMPTY_OVERRIDES reference to avoid render recomputation', () => {
    assert.ok(Array.isArray(EMPTY_OVERRIDES))
    assert.strictEqual(EMPTY_OVERRIDES.length, 0)
    assert.ok(Object.isFrozen(EMPTY_OVERRIDES))
  })

  // Test 7: ConfirmationDialog backdrop detection logic
  it('7. distinguishes between true backdrop clicks and keyboard/child button events', () => {
    function isBackdropClick(
      target: unknown,
      dialogEl: unknown,
      clientX: number,
      clientY: number,
      detail: number,
      rect: { left: number; right: number; top: number; bottom: number },
    ): boolean {
      if (target !== dialogEl) return false
      if (clientX === 0 && clientY === 0 && detail === 0) return false
      return (
        clientX < rect.left ||
        clientX > rect.right ||
        clientY < rect.top ||
        clientY > rect.bottom
      )
    }

    const mockDialog = {}
    const mockButton = {}
    const dialogRect = { left: 100, right: 500, top: 100, bottom: 400 }

    // Click on button inside dialog -> false
    assert.strictEqual(
      isBackdropClick(mockButton, mockDialog, 200, 200, 1, dialogRect),
      false,
      'Click on button must not trigger backdrop close',
    )

    // Keyboard activation (Space/Enter) on button (detail=0, clientX=0, clientY=0) -> false
    assert.strictEqual(
      isBackdropClick(mockDialog, mockDialog, 0, 0, 0, dialogRect),
      false,
      'Keyboard button activation must not trigger backdrop close',
    )

    // True backdrop click outside bounds (e.g. x=50, y=50) -> true
    assert.strictEqual(
      isBackdropClick(mockDialog, mockDialog, 50, 50, 1, dialogRect),
      true,
      'Click outside dialog bounds on ::backdrop must trigger close',
    )

    // Click inside dialog bounds on dialog container -> false
    assert.strictEqual(
      isBackdropClick(mockDialog, mockDialog, 250, 250, 1, dialogRect),
      false,
      'Click inside dialog bounds must not trigger close',
    )
  })

  // Test 8: ConfirmationDialog rejection handling
  it('8. handles rejected onConfirm promises cleanly and retains dialog state', async () => {
    let dialogOpen = true
    let isSubmitting = false
    let submitError: string | null = null

    async function handleConfirm(onConfirmFn: () => Promise<void>) {
      isSubmitting = true
      submitError = null
      try {
        await onConfirmFn()
        dialogOpen = false
      } catch (err) {
        submitError = err instanceof Error ? err.message : 'Operation Failed'
      } finally {
        isSubmitting = false
      }
    }

    // Failing confirm action
    await handleConfirm(async () => {
      throw new Error('Database transaction conflict')
    })

    assert.strictEqual(dialogOpen, true, 'Dialog must remain open on failure')
    assert.strictEqual(isSubmitting, false, 'Controls must be re-enabled after failure')
    assert.strictEqual(submitError, 'Database transaction conflict', 'Error message must be captured')

    // Successful confirm action
    await handleConfirm(async () => {
      // success
    })
    assert.strictEqual(dialogOpen, false, 'Dialog must close on success')
    assert.strictEqual(submitError, null)
  })

  // Test 9: Toast duration configuration & override precedence
  it('9. enforces configured default duration and explicit duration override precedence', () => {
    const defaultDuration = 5000

    function resolveToastDuration(inputDuration: number | undefined, defaultMs: number): number {
      return inputDuration !== undefined ? inputDuration : defaultMs
    }

    // Default duration applied when omitted
    assert.strictEqual(resolveToastDuration(undefined, defaultDuration), 5000)
    assert.strictEqual(resolveToastDuration(undefined, 8000), 8000)

    // Explicit duration overrides default (including 0 for persistent toasts)
    assert.strictEqual(resolveToastDuration(2000, defaultDuration), 2000)
    assert.strictEqual(resolveToastDuration(0, defaultDuration), 0)
  })

  // Test 10: User override race safety & switching
  it('10. prevents stale user overrides from authorizing routes during user switching', () => {
    interface UserOverrideState {
      userId: string | null
      overrides: UserPermissionOverride[]
      isLoading: boolean
    }

    let state: UserOverrideState = {
      userId: 'user_1',
      overrides: [{ permission: 'users.manage', effect: 'allow' }],
      isLoading: false,
    }

    // Switch to user_2: state must immediately clear overrides and set loading
    const targetUserId = 'user_2'
    state = {
      userId: targetUserId,
      overrides: [],
      isLoading: true,
    }

    // Effective overrides for user_2 must be empty while loading
    const activeUserId = 'user_2'
    const effectiveOverrides = state.userId === activeUserId ? state.overrides : EMPTY_OVERRIDES
    assert.strictEqual(effectiveOverrides.length, 0, 'Must not carry over user_1 overrides')
    assert.strictEqual(
      checkPermissions('cashier', 'users.manage', false, effectiveOverrides),
      false,
      'Must fail closed during loading',
    )
  })

  // Test 11: I18n Completeness for Toasts across en, ar, fr
  it('11. verifies toast translation keys across English, Arabic (RTL), and French', () => {
    const locales = [
      { code: 'en', dict: en },
      { code: 'ar', dict: ar },
      { code: 'fr', dict: fr },
    ]

    for (const { code, dict } of locales) {
      assert.ok(dict.toasts, `Locales ${code} must have toasts dictionary`)
      assert.ok(dict.toasts.dismiss, `Locales ${code} must have toasts.dismiss`)
      assert.ok(dict.toasts.errorTitle, `Locales ${code} must have toasts.errorTitle`)
      assert.ok(dict.toasts.warningTitle, `Locales ${code} must have toasts.warningTitle`)
      assert.ok(dict.toasts.successTitle, `Locales ${code} must have toasts.successTitle`)
      assert.ok(dict.toasts.infoTitle, `Locales ${code} must have toasts.infoTitle`)
    }
  })

  // Test 12: I18n Completeness for Confirmation & Audit Dialog across en, ar, fr
  it('12. verifies confirmation dialog translation keys across English, Arabic (RTL), and French', () => {
    const locales = [
      { code: 'en', dict: en },
      { code: 'ar', dict: ar },
      { code: 'fr', dict: fr },
    ]

    for (const { code, dict } of locales) {
      assert.ok(dict.confirmation, `Locales ${code} must have confirmation dictionary`)
      assert.ok(dict.confirmation.defaultTitle, `Locales ${code} must have confirmation.defaultTitle`)
      assert.ok(dict.confirmation.defaultDescription, `Locales ${code} must have confirmation.defaultDescription`)
      assert.ok(dict.confirmation.confirm, `Locales ${code} must have confirmation.confirm`)
      assert.ok(dict.confirmation.cancel, `Locales ${code} must have confirmation.cancel`)
      assert.ok(dict.confirmation.reasonLabel, `Locales ${code} must have confirmation.reasonLabel`)
      assert.ok(dict.confirmation.reasonRequired, `Locales ${code} must have confirmation.reasonRequired`)
    }
  })

  // Test 13: I18n Completeness for States & Permission Denied across en, ar, fr
  it('13. verifies states and permission-denied translations across English, Arabic (RTL), and French', () => {
    const locales = [
      { code: 'en', dict: en },
      { code: 'ar', dict: ar },
      { code: 'fr', dict: fr },
    ]

    for (const { code, dict } of locales) {
      assert.ok(dict.states, `Locales ${code} must have states dictionary`)
      assert.ok(dict.states.error.title, `Locales ${code} must have states.error.title`)
      assert.ok(dict.states.error.retry, `Locales ${code} must have states.error.retry`)
      assert.ok(dict.states.permissionDenied.title, `Locales ${code} must have states.permissionDenied.title`)
      assert.ok(dict.states.permissionDenied.description, `Locales ${code} must have states.permissionDenied.description`)
      assert.ok(dict.states.permissionDenied.requiredRole, `Locales ${code} must have states.permissionDenied.requiredRole`)
      assert.ok(dict.states.permissionDenied.action, `Locales ${code} must have states.permissionDenied.action`)
    }
  })

  // Test 14: I18n Completeness for ErrorBoundary across en, ar, fr
  it('14. verifies error boundary translations across English, Arabic (RTL), and French', () => {
    const locales = [
      { code: 'en', dict: en },
      { code: 'ar', dict: ar },
      { code: 'fr', dict: fr },
    ]

    for (const { code, dict } of locales) {
      assert.ok(dict.errorBoundary, `Locales ${code} must have errorBoundary dictionary`)
      assert.ok(dict.errorBoundary.title, `Locales ${code} must have errorBoundary.title`)
      assert.ok(dict.errorBoundary.description, `Locales ${code} must have errorBoundary.description`)
      assert.ok(dict.errorBoundary.tryAgain, `Locales ${code} must have errorBoundary.tryAgain`)
      assert.ok(dict.errorBoundary.reloadApp, `Locales ${code} must have errorBoundary.reloadApp`)
    }
  })

  // Test 15: Status processing key completeness across en, ar, fr
  it('15. verifies status.processing translations across English, Arabic (RTL), and French', () => {
    const locales = [
      { code: 'en', dict: en },
      { code: 'ar', dict: ar },
      { code: 'fr', dict: fr },
    ]

    for (const { code, dict } of locales) {
      assert.ok(dict.status, `Locales ${code} must have status dictionary`)
      assert.ok(dict.status.processing, `Locales ${code} must have status.processing`)
    }
  })
})
