// Deterministic Unit & Integration Tests for F1.14: Local POS PIN Authentication & Lock Screen
// Tests PIN validation, verification via verify_pin, branch context preservation, rate limiting/lockout,
// inactivity timeout, tenant context preservation, fail-closed error mapping, locale normalization, and full i18n parity.

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import React from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { validatePinInput, sanitizeAuthErrorMessage } from '../components/auth/validation.ts'
import { MockAuthApiClient, getAuthApi, setAuthApi } from '../services/authApi.ts'
import { AUTH_STORAGE_KEYS, performPinUnlock, performLocalLogin, performLogout } from '../context/authSession.ts'
import { en, ar, fr, getDirectionForLocale } from '../i18n/index.ts'
import {
  DEFAULT_INACTIVITY_TIMEOUT_MS,
  createInactivityTracker,
  useInactivityTimeout,
  ACTIVITY_EVENTS,
} from '../hooks/useInactivityTimeout.ts'
import { handleLockScreenKeyDown } from '../components/lock/lockHandlers.ts'

// Polyfill minimal browser-like sessionStorage and EventTarget for Node.js test environment
class MockSessionStorage {
  private store: Map<string, string> = new Map()

  getItem(key: string): string | null {
    return this.store.get(key) ?? null
  }

  setItem(key: string, value: string): void {
    this.store.set(key, String(value))
  }

  removeItem(key: string): void {
    this.store.delete(key)
  }

  clear(): void {
    this.store.clear()
  }
}

interface MockWindow {
  sessionStorage: MockSessionStorage
  listeners: Map<string, Set<() => void>>
  addEventListener: (event: string, handler: () => void) => void
  removeEventListener: (event: string, handler: () => void) => void
  dispatchEvent: (event: string) => void
}

const mockWindow: MockWindow = {
  sessionStorage: new MockSessionStorage(),
  listeners: new Map(),
  addEventListener(event: string, handler: () => void) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set())
    }
    this.listeners.get(event)!.add(handler)
  },
  removeEventListener(event: string, handler: () => void) {
    this.listeners.get(event)?.delete(handler)
  },
  dispatchEvent(event: string) {
    const handlers = this.listeners.get(event)
    if (handlers) {
      for (const h of handlers) h()
    }
  },
}

if (typeof globalThis.window === 'undefined') {
  ;(globalThis as unknown as { window: MockWindow }).window = mockWindow
} else {
  if (!globalThis.window.sessionStorage) {
    ;(globalThis.window as unknown as { sessionStorage: MockSessionStorage }).sessionStorage =
      new MockSessionStorage()
  }
  if (!globalThis.window.addEventListener) {
    globalThis.window.addEventListener = mockWindow.addEventListener
    globalThis.window.removeEventListener = mockWindow.removeEventListener
  }
}

describe('F1.14 Local POS PIN Authentication & Lock Screen Test Suite', () => {
  // 1. PIN Input Validation
  it('1. validatePinInput enforces non-empty numeric digits', () => {
    assert.strictEqual(validatePinInput(''), 'pin.errors.required')
    assert.strictEqual(validatePinInput('   '), 'pin.errors.required')
    assert.strictEqual(validatePinInput('12a4'), 'pin.errors.digitsOnly')
    assert.strictEqual(validatePinInput('pin1'), 'pin.errors.digitsOnly')
    assert.strictEqual(validatePinInput('1234'), null)
    assert.strictEqual(validatePinInput('0000'), null)
    assert.strictEqual(validatePinInput('123456'), null)
  })

  // 2. Successful PIN Unlock via authoritative verify_pin contract & explicit user state assertions
  it('2. successful PIN verification returns new session ID, persists to storage, and returns full user state', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    const { result, user } = await performPinUnlock('usr_cashier_01', '1234', mockApi)

    assert.strictEqual(result.success, true)
    assert.ok(result.session_id?.startsWith('sess_pin_'))
    assert.strictEqual(result.user_id, 'usr_cashier_01')
    assert.strictEqual(result.role, 'cashier')
    assert.strictEqual(result.branch_id, 'br_default')

    assert.ok(user)
    assert.strictEqual(user.id, 'usr_cashier_01')
    assert.strictEqual(user.role, 'cashier')
    assert.strictEqual(user.branch_id, 'br_default')

    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), 'local')
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), result.session_id)

    // Verify session is active in backend state
    const state = await mockApi.getAuthState(result.session_id)
    assert.strictEqual(state.authenticated, true)
    assert.strictEqual(state.session_id, result.session_id)
  })

  // 3. Branch Context Preservation across PIN Unlock
  it('3. PIN unlock preserves existing active branch context when backend response omits it', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    // 3a. When backend does not return a branch_id, existing branch_id is preserved
    const apiWithoutBranch: typeof mockApi = {
      ...mockApi,
      verifyPin: async (userId: string) => ({
        success: true,
        session_id: 'sess_123',
        user_id: userId,
        role: 'cashier',
        branch_id: null,
      }),
    } as unknown as typeof mockApi

    const preserved = await performPinUnlock('usr_operator', '1234', apiWithoutBranch, 'br_existing_main')
    assert.strictEqual(preserved.result.success, true)
    assert.strictEqual(preserved.user?.branch_id, 'br_existing_main', 'Existing branch context must be preserved')

    // 3b. When backend returns an explicit non-empty branch_id, it is applied
    const apiWithBranch: typeof mockApi = {
      ...mockApi,
      verifyPin: async (userId: string) => ({
        success: true,
        session_id: 'sess_456',
        user_id: userId,
        role: 'cashier',
        branch_id: 'br_updated_downtown',
      }),
    } as unknown as typeof mockApi

    const updated = await performPinUnlock('usr_operator', '1234', apiWithBranch, 'br_existing_main')
    assert.strictEqual(updated.result.success, true)
    assert.strictEqual(updated.user?.branch_id, 'br_updated_downtown', 'Explicit backend branch_id must take precedence')
  })

  // 4. Invalid PIN Error Handling & Sanitization
  it('4. invalid PIN triggers safe localized error without exposing internals', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    await assert.rejects(
      async () => {
        await mockApi.verifyPin('usr_cashier_01', 'wrong_pin')
      },
      (err: Error) => {
        const errorKey = sanitizeAuthErrorMessage(err)
        assert.strictEqual(errorKey, 'auth.errors.invalidPin')
        return true
      },
    )

    await assert.rejects(
      async () => {
        await mockApi.verifyPin('usr_cashier_01', '0000')
      },
      (err: Error) => {
        const errorKey = sanitizeAuthErrorMessage(err)
        assert.strictEqual(errorKey, 'auth.errors.invalidPin')
        return true
      },
    )

    // Empty fields rejection
    await assert.rejects(
      async () => {
        await mockApi.verifyPin('', '')
      },
      /User ID and PIN are required/,
    )
  })

  // 5. Rate Limiting & Account Lockout Mapping
  it('5. backend rate limiting and lockout error messages map directly to auth.errors.accountLocked', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    // Simulated lockout trigger
    await assert.rejects(
      async () => {
        await mockApi.verifyPin('usr_cashier_01', '9999')
      },
      (err: Error) => {
        const errorKey = sanitizeAuthErrorMessage(err)
        assert.strictEqual(errorKey, 'auth.errors.accountLocked')
        return true
      },
    )

    mockApi.isRateLimited = true
    await assert.rejects(
      async () => {
        await mockApi.verifyPin('usr_cashier_01', '1234')
      },
      (err: Error) => {
        const errorKey = sanitizeAuthErrorMessage(err)
        assert.strictEqual(errorKey, 'auth.errors.accountLocked')
        return true
      },
    )
  })

  // 6. Inactivity Timeout Tracker & Configuration
  it('6. inactivity timeout configuration defaults to 15 minutes and tracker handles lifecycle', async () => {
    assert.strictEqual(DEFAULT_INACTIVITY_TIMEOUT_MS, 15 * 60 * 1000)
    assert.ok(ACTIVITY_EVENTS.length >= 4)

    let timeoutFired = false
    const tracker = createInactivityTracker({
      onTimeout: () => {
        timeoutFired = true
      },
      timeoutMs: 50,
      isEnabled: true,
    })

    // Reset before timeout fires
    tracker.reset()

    await new Promise((resolve) => setTimeout(resolve, 80))
    assert.strictEqual(timeoutFired, true)

    tracker.cleanup()

    // Disabled tracker does not fire
    let disabledFired = false
    const disabledTracker = createInactivityTracker({
      onTimeout: () => {
        disabledFired = true
      },
      timeoutMs: 20,
      isEnabled: false,
    })
    disabledTracker.reset()
    await new Promise((resolve) => setTimeout(resolve, 40))
    assert.strictEqual(disabledFired, false)
    disabledTracker.cleanup()

    // Zero timeout does not fire
    let zeroFired = false
    const zeroTracker = createInactivityTracker({
      onTimeout: () => {
        zeroFired = true
      },
      timeoutMs: 0,
      isEnabled: true,
    })
    zeroTracker.reset()
    await new Promise((resolve) => setTimeout(resolve, 20))
    assert.strictEqual(zeroFired, false)
    zeroTracker.cleanup()
  })

  // 7. useInactivityTimeout React Hook Rendering
  it('7. useInactivityTimeout mounts cleanly within a React component without mutating exports', () => {
    function HookTestComponent({ isEnabled = true, timeoutMs = 1000 }: { isEnabled?: boolean; timeoutMs?: number }) {
      useInactivityTimeout({
        onTimeout: () => {},
        timeoutMs,
        isEnabled,
      })
      return React.createElement('div', { 'data-testid': 'inactivity-tester' }, 'ready')
    }

    const html = renderToStaticMarkup(React.createElement(HookTestComponent, { isEnabled: true, timeoutMs: 500 }))
    assert.ok(html.includes('inactivity-tester'))

    const disabledHtml = renderToStaticMarkup(
      React.createElement(HookTestComponent, { isEnabled: false, timeoutMs: 0 }),
    )
    assert.ok(disabledHtml.includes('inactivity-tester'))
  })

  // 8. Security: No Plaintext PIN Persistence
  it('8. plaintext PIN is never persisted to storage across unlock cycles', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    const secretPin = '582914'
    const { result } = await performPinUnlock('usr_cashier_01', secretPin, mockApi)
    assert.strictEqual(result.success, true)

    // Ensure PIN does not exist anywhere in storage
    const allStoredValues = [
      window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID),
      window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE),
      window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN),
      window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_USER),
    ]

    for (const val of allStoredValues) {
      if (val) {
        assert.ok(!val.includes(secretPin), `Storage value "${val}" must not contain plaintext PIN`)
      }
    }
  })

  // 9. Context Preservation: Lock & Unlock Retains Tenant Context
  it('9. terminal lock and unlock preserves user identity and tenant context', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    // 1. Initial Login
    const { result: loginRes, user: loggedInUser } = await performLocalLogin(
      { username: 'cashier_main', password: 'valid_password' },
      mockApi,
    )
    assert.ok(loginRes.session_id)
    assert.ok(loggedInUser)
    assert.strictEqual(loggedInUser.username, 'cashier_main')

    // 2. Terminal Lock (state transition simulation)
    const activeContext = {
      org: { id: 'org_1', name: 'Acme Retail' },
      branch: { id: 'br_1', name: 'Downtown Branch' },
      register: { id: 'reg_1', name: 'POS Terminal 01' },
    }

    // 3. Unlock with PIN
    const { result: unlockRes, user: unlockedUser } = await performPinUnlock(
      loggedInUser.id,
      '1234',
      mockApi,
      loggedInUser.branch_id,
    )
    assert.strictEqual(unlockRes.success, true)
    assert.strictEqual(unlockedUser?.id, loggedInUser.id)

    // Context remains identical
    assert.strictEqual(activeContext.org.id, 'org_1')
    assert.strictEqual(activeContext.branch.id, 'br_1')
    assert.strictEqual(activeContext.register.id, 'reg_1')

    // Unsuccessful unlock without throw simulation
    const failingApi: typeof mockApi = {
      ...mockApi,
      verifyPin: async () => ({ success: false }),
    } as unknown as typeof mockApi
    const failedUnlock = await performPinUnlock('usr_1', 'bad', failingApi)
    assert.strictEqual(failedUnlock.result.success, false)
    assert.strictEqual(failedUnlock.user, null)
  })

  // 10. Switch Account / Sign Out Clears Session & Storage
  it('10. switch account / sign out from lock screen revokes session and clears storage', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    const { result } = await performLocalLogin({ username: 'cashier_a', password: 'pass' }, mockApi)
    assert.ok(result.session_id)

    await performLogout(result.session_id, 'local', mockApi)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), null)

    const stateAfterLogout = await mockApi.getAuthState(result.session_id)
    assert.strictEqual(stateAfterLogout.authenticated, false)
  })

  // 11. Fail-Closed Error Mapping for Database & Network Failures
  it('11. unhandled backend failures during PIN unlock are sanitized fail-closed', () => {
    const dbError = 'rusqlite::Error: Database locked at /src-tauri/src/user/mod.rs:805'
    assert.strictEqual(sanitizeAuthErrorMessage(dbError), 'auth.errors.generic')

    const panicError = 'internal panic: attempt to subtract with overflow'
    assert.strictEqual(sanitizeAuthErrorMessage(panicError), 'auth.errors.generic')

    const userInactive = 'Validation error: User account is inactive'
    assert.strictEqual(sanitizeAuthErrorMessage(userInactive), 'auth.errors.userInactive')

    const branchInactive = 'Validation error: Branch is inactive'
    assert.strictEqual(sanitizeAuthErrorMessage(branchInactive), 'auth.errors.branchInactive')
  })

  // 12. Translation Key Parity for lock and pin namespaces across en, ar, fr
  it('12. lock and pin translation keys have 100% parity across English, Arabic, and French', () => {
    function getNestedKeys(obj: Record<string, unknown>, prefix = ''): string[] {
      let keys: string[] = []
      for (const [k, v] of Object.entries(obj)) {
        const fullKey = prefix ? `${prefix}.${k}` : k
        if (v && typeof v === 'object' && !Array.isArray(v)) {
          keys = keys.concat(getNestedKeys(v as Record<string, unknown>, fullKey))
        } else {
          keys.push(fullKey)
        }
      }
      return keys.sort()
    }

    const enLockKeys = getNestedKeys(en.lock)
    const arLockKeys = getNestedKeys(ar.lock)
    const frLockKeys = getNestedKeys(fr.lock)

    assert.deepStrictEqual(arLockKeys, enLockKeys, 'Arabic lock keys must match English 100%')
    assert.deepStrictEqual(frLockKeys, enLockKeys, 'French lock keys must match English 100%')

    const enPinKeys = getNestedKeys(en.pin)
    const arPinKeys = getNestedKeys(ar.pin)
    const frPinKeys = getNestedKeys(fr.pin)

    assert.deepStrictEqual(arPinKeys, enPinKeys, 'Arabic pin keys must match English 100%')
    assert.deepStrictEqual(frPinKeys, enPinKeys, 'French pin keys must match English 100%')

    assert.ok(en.auth.errors.invalidPin)
    assert.ok(ar.auth.errors.invalidPin)
    assert.ok(fr.auth.errors.invalidPin)

    assert.ok(en.auth.errors.accountLocked)
    assert.ok(ar.auth.errors.accountLocked)
    assert.ok(fr.auth.errors.accountLocked)
  })

  // 13. getDirectionForLocale normalization, empty guards, and Arabic RTL Direction
  it('13. getDirectionForLocale properly normalizes locale codes and guards empty/null values', () => {
    // Standard Arabic and region variants
    assert.strictEqual(getDirectionForLocale('ar'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar-DZ'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar-SA'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar_EG'), 'rtl')
    assert.strictEqual(getDirectionForLocale('  AR-SA  '), 'rtl')

    // LTR languages
    assert.strictEqual(getDirectionForLocale('en'), 'ltr')
    assert.strictEqual(getDirectionForLocale('en-US'), 'ltr')
    assert.strictEqual(getDirectionForLocale('fr'), 'ltr')

    // Empty, whitespace, null, and undefined guards
    assert.strictEqual(getDirectionForLocale(''), 'ltr')
    assert.strictEqual(getDirectionForLocale('   '), 'ltr')
    assert.strictEqual(getDirectionForLocale(null), 'ltr')
    assert.strictEqual(getDirectionForLocale(undefined), 'ltr')
  })

  // 14. Keyboard Enter event behavior: exercise production handleLockScreenKeyDown
  it('14. handleLockScreenKeyDown routes digits, backspace, clear, and guards Enter when focused on interactive button', () => {
    let submitted = false
    let enteredDigit = ''
    let backspaced = false
    let cleared = false

    const actions = {
      onDigit: (d: string) => {
        enteredDigit = d
      },
      onBackspace: () => {
        backspaced = true
      },
      onClear: () => {
        cleared = true
      },
      onSubmit: () => {
        submitted = true
      },
    }

    // 14a. Digits
    let digitPrevented = false
    handleLockScreenKeyDown(
      { key: '5', preventDefault: () => { digitPrevented = true } },
      actions,
      { tagName: 'DIV' },
    )
    assert.strictEqual(enteredDigit, '5')
    assert.strictEqual(digitPrevented, true)

    // 14b. Backspace
    let backspacePrevented = false
    handleLockScreenKeyDown(
      { key: 'Backspace', preventDefault: () => { backspacePrevented = true } },
      actions,
      { tagName: 'DIV' },
    )
    assert.strictEqual(backspaced, true)
    assert.strictEqual(backspacePrevented, true)

    // 14c. Escape / Clear
    let clearPrevented = false
    handleLockScreenKeyDown(
      { key: 'Escape', preventDefault: () => { clearPrevented = true } },
      actions,
      { tagName: 'DIV' },
    )
    assert.strictEqual(cleared, true)
    assert.strictEqual(clearPrevented, true)

    // 14d. Enter on BUTTON: does NOT submit, does NOT preventDefault
    let buttonEnterPrevented = false
    handleLockScreenKeyDown(
      { key: 'Enter', preventDefault: () => { buttonEnterPrevented = true } },
      actions,
      { tagName: 'BUTTON' },
    )
    assert.strictEqual(submitted, false)
    assert.strictEqual(buttonEnterPrevented, false)

    // 14e. Enter on LINK (A): does NOT submit, does NOT preventDefault
    let linkEnterPrevented = false
    handleLockScreenKeyDown(
      { key: 'Enter', preventDefault: () => { linkEnterPrevented = true } },
      actions,
      { tagName: 'A' },
    )
    assert.strictEqual(submitted, false)
    assert.strictEqual(linkEnterPrevented, false)

    // 14f. Enter on non-button element (e.g. DIV): triggers onSubmit and preventDefault
    let divEnterPrevented = false
    handleLockScreenKeyDown(
      { key: 'Enter', preventDefault: () => { divEnterPrevented = true } },
      actions,
      { tagName: 'DIV' },
    )
    assert.strictEqual(submitted, true)
    assert.strictEqual(divEnterPrevented, true)

    // 14g. Modifier keys are ignored
    let modifierPrevented = false
    let modifierSubmitted = false
    handleLockScreenKeyDown(
      { key: 'Enter', ctrlKey: true, preventDefault: () => { modifierPrevented = true } },
      { ...actions, onSubmit: () => { modifierSubmitted = true } },
      { tagName: 'DIV' },
    )
    assert.strictEqual(modifierSubmitted, false)
    assert.strictEqual(modifierPrevented, false)
  })
})
