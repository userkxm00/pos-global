// Deterministic Unit & Lifecycle Tests for F1.13 Authentication & Session Management
// Covers online/local login, session restoration, online/local logout revocation, validation, error sanitization, tenant isolation, and i18n.

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import {
  validateOnlineInput,
  validateLocalInput,
  sanitizeAuthErrorMessage,
} from '../components/auth/validation.ts'
import {
  MockAuthApiClient,
  getAuthApi,
  setAuthApi,
  extractInvokeErrorMessage,
  getDefaultSupabaseConfig,
  stripTrailingSlash,
} from '../services/authApi.ts'
import {
  AUTH_STORAGE_KEYS,
  performOnlineLogin,
  performTokenRefresh,
  performSingleFlightRefresh,
  isTokenExpiringSoon,
  storeOnlineSession,
  performLocalLogin,
  performLogout,
  evaluateStoredSession,
  restoreOnlineSession,
  restoreLocalSession,
  clearStoredAuth,
} from '../context/authSession.ts'
import { classifyAuthError, createTypedAuthError } from '../services/authApi.ts'
import { en, ar, fr, getDirectionForLocale } from '../i18n/index.ts'

// Polyfill minimal browser-like sessionStorage for Node.js test environment
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

// Attach mock sessionStorage to global window if missing
if (typeof globalThis.window === 'undefined') {
  ;(globalThis as unknown as { window: { sessionStorage: MockSessionStorage } }).window = {
    sessionStorage: new MockSessionStorage(),
  }
} else if (!globalThis.window.sessionStorage) {
  ;(globalThis.window as unknown as { sessionStorage: MockSessionStorage }).sessionStorage = new MockSessionStorage()
}

describe('F1.13 Authentication & Session Lifecycle Test Suite', () => {
  // 1. Online Supabase Login: Success
  it('1. successful online login establishes cloud session and persists session storage', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    const { session, user } = await performOnlineLogin(
      { email: 'admin@posglobal.com', password: 'valid_password_123' },
      mockApi,
    )

    assert.ok(session.access_token)
    assert.strictEqual(session.user.email, 'admin@posglobal.com')
    assert.strictEqual(user.id, session.user.id)
    assert.strictEqual(user.role, 'owner')

    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), 'online')
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), session.access_token)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), session.user.id)
  })

  // 2. Online Supabase Login: Invalid Credentials & Edge Cases
  it('2. online login with invalid credentials or missing fields fails with safe error', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    await assert.rejects(
      async () => {
        await mockApi.onlineLogin({
          email: 'admin@posglobal.com',
          password: 'wrong_password',
        })
      },
      (err: Error) => {
        const errorKey = sanitizeAuthErrorMessage(err)
        assert.strictEqual(errorKey, 'auth.errors.invalidCredentials')
        return true
      },
    )

    await assert.rejects(
      async () => {
        await mockApi.onlineLogin({ email: '', password: '' })
      },
      /Email and password are required/,
    )

    mockApi.shouldFailWith = 'Network error: unreachable'
    await assert.rejects(
      async () => {
        await mockApi.onlineLogin({ email: 'admin@test.com', password: 'pass' })
      },
      /Network error/,
    )
  })

  // 3. Local POS User Login: Success
  it('3. successful local login establishes local session and persists session ID', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    const { result, user } = await performLocalLogin(
      { username: 'cashier_1', password: 'secret_pos_password' },
      mockApi,
    )

    assert.strictEqual(result.success, true)
    assert.ok(result.session_id?.startsWith('sess_'))
    assert.strictEqual(result.role, 'admin')
    assert.strictEqual(user?.username, 'cashier_1')

    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), 'local')
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), result.session_id)

    // Verify session is active in state
    const authState = await mockApi.getAuthState(result.session_id)
    assert.strictEqual(authState.authenticated, true)
    assert.strictEqual(authState.session_id, result.session_id)
  })

  // 4. Local POS User Login: Invalid Credentials & Edge Cases
  it('4. local login with incorrect password or empty inputs fails cleanly', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    await assert.rejects(
      async () => {
        await mockApi.localLogin({
          username: 'cashier_1',
          password: 'wrong_password',
        })
      },
      (err: Error) => {
        const errorKey = sanitizeAuthErrorMessage(err)
        assert.strictEqual(errorKey, 'auth.errors.invalidCredentials')
        return true
      },
    )

    await assert.rejects(
      async () => {
        await mockApi.localLogin({ username: '', password: '' })
      },
      /Username and password are required/,
    )

    mockApi.shouldFailWith = 'Database locked'
    await assert.rejects(
      async () => {
        await mockApi.localLogin({ username: 'admin', password: 'password' })
      },
      /Database locked/,
    )
  })

  // 5. Input Validation
  it('5. validates online and local input fields', () => {
    // Online validation
    const emptyOnline = validateOnlineInput({})
    assert.strictEqual(emptyOnline.email, 'auth.validation.emailRequired')
    assert.strictEqual(emptyOnline.password, 'auth.validation.passwordRequired')

    const invalidEmail = validateOnlineInput({ email: 'not-an-email', password: 'pass' })
    assert.strictEqual(invalidEmail.email, 'auth.validation.emailInvalid')

    const validOnline = validateOnlineInput({ email: 'user@example.com', password: 'validpassword' })
    assert.strictEqual(Object.keys(validOnline).length, 0)

    // Local validation
    const emptyLocal = validateLocalInput({})
    assert.strictEqual(emptyLocal.username, 'auth.validation.usernameRequired')
    assert.strictEqual(emptyLocal.password, 'auth.validation.passwordRequired')

    const validLocal = validateLocalInput({ username: 'admin', password: 'password123' })
    assert.strictEqual(Object.keys(validLocal).length, 0)
  })

  // 6. Fail-Closed Error Sanitization
  it('6. error sanitization maps domain errors and masks backend/database internals', () => {
    assert.strictEqual(
      sanitizeAuthErrorMessage('Invalid credentials: Email and password are required'),
      'auth.errors.invalidCredentials',
    )
    assert.strictEqual(
      sanitizeAuthErrorMessage('Rate limit exceeded: Please wait'),
      'auth.errors.rateLimited',
    )
    assert.strictEqual(
      sanitizeAuthErrorMessage('Network error: connection refused'),
      'auth.errors.networkError',
    )
    assert.strictEqual(
      sanitizeAuthErrorMessage('Session has expired'),
      'auth.errors.sessionExpired',
    )
    assert.strictEqual(
      sanitizeAuthErrorMessage('Session has been revoked'),
      'auth.errors.sessionRevoked',
    )
    assert.strictEqual(
      sanitizeAuthErrorMessage('User account is inactive'),
      'auth.errors.userInactive',
    )
    assert.strictEqual(
      sanitizeAuthErrorMessage('Branch is inactive'),
      'auth.errors.branchInactive',
    )
    assert.strictEqual(
      sanitizeAuthErrorMessage('Service unavailable'),
      'auth.errors.serviceUnavailable',
    )
    assert.strictEqual(
      sanitizeAuthErrorMessage('Configuration error'),
      'auth.errors.configurationError',
    )

    // Unsafe raw SQLite / panic / internal error is masked to generic localized key
    const rawSqlError = 'sqlite error: syntax error near SELECT * FROM users'
    assert.strictEqual(sanitizeAuthErrorMessage(rawSqlError), 'auth.errors.generic')

    const panicError = 'internal panic at auth/mod.rs:42:15'
    assert.strictEqual(sanitizeAuthErrorMessage(panicError), 'auth.errors.generic')

    assert.strictEqual(sanitizeAuthErrorMessage(null), 'auth.errors.unknown')
    assert.strictEqual(sanitizeAuthErrorMessage(undefined), 'auth.errors.unknown')
    assert.strictEqual(sanitizeAuthErrorMessage({ message: 'network error: failed' }), 'auth.errors.networkError')
  })

  // 7. Session Validation & Expiry
  it('7. missing or non-existent session ID reports unauthenticated state', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    const nullState = await mockApi.getAuthState(null)
    assert.strictEqual(nullState.authenticated, false)

    const missingState = await mockApi.getAuthState('sess_non_existent')
    assert.strictEqual(missingState.authenticated, false)

    mockApi.shouldFailWith = 'Database error'
    await assert.rejects(
      async () => {
        await mockApi.getAuthState('sess_any')
      },
      /Database error/,
    )
  })

  // 8. Explicit Logout & Failure Tolerance (Online Cloud & Local POS)
  it('8. logout successfully revokes active local/online sessions and handles remote errors fail-closed', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    // 8a. Local Logout: Success
    const { result } = await performLocalLogin({ username: 'operator', password: 'password123' }, mockApi)
    assert.ok(result.session_id)

    const stateBefore = await mockApi.getAuthState(result.session_id)
    assert.strictEqual(stateBefore.authenticated, true)

    await performLogout(result.session_id, 'local', mockApi)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), null)

    const stateAfter = await mockApi.getAuthState(result.session_id)
    assert.strictEqual(stateAfter.authenticated, false)

    // 8b. Online Logout: Success & Cloud Revocation
    const { session } = await performOnlineLogin(
      { email: 'owner@posglobal.com', password: 'secret_password_123' },
      mockApi,
    )
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), session.access_token)

    await performLogout(session.user.id, 'online', mockApi)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), null)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), null)
    assert.ok(mockApi.revokedCloudTokens.has(session.access_token))

    // Null token online logout noop
    await mockApi.onlineLogout(null)

    // 8c. Local Logout with Remote Error: Fail-Closed
    mockApi.shouldFailWith = 'Remote logout failure'
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.SESSION_ID, 'sess_throw')
    await performLogout('sess_throw', 'local', mockApi)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), null)

    // 8d. Online Logout with Remote Error: Fail-Closed
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, 'tok_throw')
    await performLogout('usr_cloud', 'online', mockApi, 'tok_throw')
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), null)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), null)
  })

  // 9. Session Persistence & Restoration Lifecycle
  it('9. evaluates and restores online and local sessions from storage', async () => {
    const storage = window.sessionStorage
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    // 9a. Test Local Session Evaluation
    storage.clear()
    storage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'local')
    storage.setItem(AUTH_STORAGE_KEYS.SESSION_ID, 'sess_local_456')

    mockApi.activeSessions.set('sess_local_456', {
      authenticated: true,
      session_id: 'sess_local_456',
      user_id: 'usr_cashier_456',
      branch_id: 'br_main',
      role: 'cashier',
    })

    const localRestored = await evaluateStoredSession(mockApi)
    assert.strictEqual(localRestored.status, 'authenticated')
    assert.strictEqual(localRestored.user?.id, 'usr_cashier_456')
    assert.strictEqual(localRestored.mode, 'local')

    // 9b. Test Expired Local Session
    storage.setItem(AUTH_STORAGE_KEYS.SESSION_ID, 'sess_expired')
    const expiredLocal = await evaluateStoredSession(mockApi)
    assert.strictEqual(expiredLocal.status, 'expired')

    // 9c. Test Local Session API Throw
    mockApi.shouldFailWith = 'Database error'
    const errorLocal = await restoreLocalSession('sess_throw', mockApi)
    assert.strictEqual(errorLocal.status, 'expired')
    mockApi.shouldFailWith = null

    // 9d. Test Online Session Evaluation
    storage.clear()
    storage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
    storage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, 'jwt_token_789')
    storage.setItem(
      AUTH_STORAGE_KEYS.CLOUD_USER,
      JSON.stringify({ id: 'usr_cloud_789', email: 'owner@posglobal.com', role: 'owner' }),
    )

    const onlineRestored = await evaluateStoredSession(mockApi)
    assert.strictEqual(onlineRestored.status, 'authenticated')
    assert.strictEqual(onlineRestored.user?.email, 'owner@posglobal.com')
    assert.strictEqual(onlineRestored.mode, 'online')

    // 9e. Test Corrupt Online Storage Data
    const corruptOnline = await restoreOnlineSession('token', 'not-json')
    assert.strictEqual(corruptOnline.status, 'expired')

    const emptyOnline = await restoreOnlineSession('', null)
    assert.strictEqual(emptyOnline.status, 'unauthenticated')

    // 9f. Clear Auth
    clearStoredAuth()
    assert.strictEqual(storage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), null)

    // 9g. Empty storage evaluate
    const emptyStorageRestored = await evaluateStoredSession(mockApi)
    assert.strictEqual(emptyStorageRestored.status, 'unauthenticated')
  })

  // 10. Stale Tenant / User Isolation on User Switch & Unsuccessful Local Login
  it('10. user A logout followed by user B login does not leak stale user context', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    // User A Login
    const userA = await performLocalLogin({ username: 'user_a', password: 'password123' }, mockApi)
    assert.strictEqual(userA.user?.username, 'user_a')

    // User A Logout
    await performLogout(userA.result.session_id!, 'local', mockApi)
    const stateAfterLogout = await mockApi.getAuthState(userA.result.session_id)
    assert.strictEqual(stateAfterLogout.authenticated, false)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), null)

    // User B Login
    const userB = await performLocalLogin({ username: 'user_b', password: 'password123' }, mockApi)
    assert.strictEqual(userB.user?.username, 'user_b')
    assert.notStrictEqual(userB.result.session_id, userA.result.session_id)

    // Unsuccessful Login simulation (e.g. invalid response without throwing)
    const failClient: typeof mockApi = {
      ...mockApi,
      localLogin: async () => ({ success: false }),
    } as unknown as typeof mockApi
    const failedResult = await performLocalLogin({ username: 'bad', password: 'bad' }, failClient)
    assert.strictEqual(failedResult.result.success, false)
    assert.strictEqual(failedResult.user, null)
  })

  // 11. Mock Client Safety, Error Extractors, Helpers & Config Defaults
  it('11. getAuthApi provides MockAuthApiClient in test environment and extracts error messages', () => {
    const api = getAuthApi()
    assert.ok(api instanceof MockAuthApiClient)

    const strError = extractInvokeErrorMessage('simple error')
    assert.strictEqual(strError, 'simple error')

    const objError = extractInvokeErrorMessage(new Error('error instance message'))
    assert.strictEqual(objError, 'error instance message')

    const rawObj = extractInvokeErrorMessage({ code: 500 })
    assert.strictEqual(rawObj, '[object Object]')

    const cfg = getDefaultSupabaseConfig()
    assert.ok(cfg.url)
    assert.ok(cfg.publishable_key)

    assert.strictEqual(stripTrailingSlash('https://example.com/'), 'https://example.com')
    assert.strictEqual(stripTrailingSlash('https://example.com///'), 'https://example.com')
    assert.strictEqual(stripTrailingSlash('https://example.com'), 'https://example.com')
  })

  // 12. Translation Key Parity across en, ar, fr
  it('12. auth translation keys have 100% parity across English, Arabic, and French including modes.title', () => {
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

    const enKeys = getNestedKeys(en.auth)
    const arKeys = getNestedKeys(ar.auth)
    const frKeys = getNestedKeys(fr.auth)

    assert.deepStrictEqual(arKeys, enKeys, 'Arabic auth keys must match English 100%')
    assert.deepStrictEqual(frKeys, enKeys, 'French auth keys must match English 100%')
    assert.ok(en.auth.modes.title)
    assert.ok(ar.auth.modes.title)
    assert.ok(fr.auth.modes.title)
  })

  // 13. Arabic RTL Direction Support
  it('13. Arabic locale returns RTL direction for auth view layout', () => {
    assert.strictEqual(getDirectionForLocale('ar'), 'rtl')
    assert.strictEqual(getDirectionForLocale('ar-SA'), 'rtl')
    assert.strictEqual(getDirectionForLocale('en'), 'ltr')
    assert.strictEqual(getDirectionForLocale('fr'), 'ltr')
  })

  // 14. Token Expiry & Expiring-Soon Detection (F1.19)
  it('14. isTokenExpiringSoon correctly detects impending expiration threshold', () => {
    const nowSeconds = Math.floor(Date.now() / 1000)

    // Missing expiresAt -> expires soon (fail-closed)
    assert.strictEqual(isTokenExpiringSoon(undefined), true)
    assert.strictEqual(isTokenExpiringSoon(null), true)

    // Expired in the past
    assert.strictEqual(isTokenExpiringSoon(nowSeconds - 100), true)

    // Expiring in 2 minutes (threshold is 5 minutes = 300s)
    assert.strictEqual(isTokenExpiringSoon(nowSeconds + 120), true)

    // Expiring in 10 minutes (not expiring soon)
    assert.strictEqual(isTokenExpiringSoon(nowSeconds + 600), false)
  })

  // 15. Successful Token Refresh Lifecycle (F1.19)
  it('15. performTokenRefresh refreshes cloud tokens and updates session storage atomically', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    const initialUser = {
      id: 'usr_cloud_123',
      email: 'owner@example.com',
      role: 'owner',
    }

    const { session, user } = await performTokenRefresh(
      'mock_valid_refresh_token',
      mockApi,
      initialUser,
    )

    assert.ok(session.access_token)
    assert.ok(session.refresh_token)
    assert.strictEqual(user.id, 'usr_cloud_123')
    assert.strictEqual(user.role, 'owner')

    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), 'online')
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), session.access_token)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN), session.refresh_token)
  })

  // 16. Failed Token Refresh Fail-Closed Handling (F1.19)
  it('16. performTokenRefresh with invalid or expired refresh token throws and allows fail-closed handling', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    await assert.rejects(
      async () => {
        await performTokenRefresh('expired_refresh', mockApi)
      },
      (err: Error) => {
        assert.ok(err.message.includes('Session expired') || err.message.includes('invalid'))
        return true
      },
    )
  })

  // 17. Network Error Resilience during Token Refresh (F1.19)
  it('17. transient network errors during token refresh throw clear typed error without crashing', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)

    await assert.rejects(
      async () => {
        await performTokenRefresh('network_error', mockApi)
      },
      (err: Error) => {
        assert.ok(err.message.includes('Network error'))
        return true
      },
    )
  })

  // 18. Online Logout Cloud Token Revocation (F1.19)
  it('18. onlineLogout tracks revoked tokens in MockAuthApiClient and cleans storage', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, 'token_to_revoke_123')

    await performLogout('sess_123', 'online', mockApi, 'token_to_revoke_123')

    assert.ok(mockApi.revokedCloudTokens.has('token_to_revoke_123'))
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), null)
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), null)
  })

  // 19. restoreOnlineSession handles valid, expired, missing, and malformed expiration timestamps (F1.19)
  it('19. restoreOnlineSession handles valid, expired, missing, null, and malformed expiration values', async () => {
    const validUser = JSON.stringify({ id: 'usr_1', email: 'u@example.com', role: 'owner' })
    const nowSeconds = Math.floor(Date.now() / 1000)

    // Valid future expiration -> authenticated
    const future = await restoreOnlineSession('tok_123', validUser, 'ref_123', String(nowSeconds + 3600))
    assert.strictEqual(future.status, 'authenticated')
    assert.strictEqual(future.expiresAt, nowSeconds + 3600)
    assert.strictEqual(future.refreshToken, 'ref_123')

    // Already expired in past -> expired (fail-closed)
    const past = await restoreOnlineSession('tok_123', validUser, 'ref_123', String(nowSeconds - 100))
    assert.strictEqual(past.status, 'expired')

    // Missing / null string / undefined string -> authenticated with null expiresAt
    const nullStr = await restoreOnlineSession('tok_123', validUser, 'ref_123', 'null')
    assert.strictEqual(nullStr.status, 'authenticated')
    assert.strictEqual(nullStr.expiresAt, null)

    const undefStr = await restoreOnlineSession('tok_123', validUser, 'ref_123', 'undefined')
    assert.strictEqual(undefStr.status, 'authenticated')
    assert.strictEqual(undefStr.expiresAt, null)

    // Non-numeric / NaN -> null expiresAt
    const invalidNum = await restoreOnlineSession('tok_123', validUser, 'ref_123', 'not_a_number')
    assert.strictEqual(invalidNum.status, 'authenticated')
    assert.strictEqual(invalidNum.expiresAt, null)
  })

  // 20. storeOnlineSession derives expires_at from expires_in and clears omitted fields (F1.19)
  it('20. storeOnlineSession derives expires_at from expires_in and cleans up omitted fields without storing null', () => {
    window.sessionStorage.clear()

    const user = { id: 'usr_store_1', email: 'store@example.com', role: 'owner' }

    // Seed a stale refresh token so the removal branch is observable
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN, 'stale_ref')

    // Session with only expires_in
    storeOnlineSession(
      {
        access_token: 'tok_in_only',
        expires_in: 1800,
        user: { id: 'usr_store_1', email: 'store@example.com' },
      },
      user,
    )

    const storedExpiresAt = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT)
    assert.ok(storedExpiresAt)
    assert.ok(Number(storedExpiresAt) > Math.floor(Date.now() / 1000))
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN), null)

    // Subsequent session omitting expiration removes CLOUD_EXPIRES_AT
    storeOnlineSession(
      {
        access_token: 'tok_no_exp',
        user: { id: 'usr_store_1', email: 'store@example.com' },
      },
      user,
    )

    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT), null)
  })

  // 21. TypedAuthError classification (F1.19)
  it('21. classifyAuthError deterministically maps error structures and messages to AuthErrorCode', () => {
    const preclassified = createTypedAuthError('rate_limit', 'Too many requests')
    assert.strictEqual(classifyAuthError(preclassified).code, 'rate_limit')

    assert.strictEqual(classifyAuthError('Session expired: Please sign in again').code, 'session_expired')
    assert.strictEqual(classifyAuthError('Invalid Refresh Token: Refresh Token Not Found').code, 'session_expired')
    assert.strictEqual(classifyAuthError('Refresh token has already used').code, 'session_expired')
    assert.strictEqual(classifyAuthError('Rate limit exceeded: Please wait').code, 'rate_limit')
    assert.strictEqual(classifyAuthError('Network error: Unable to reach Supabase').code, 'network_error')
    assert.strictEqual(classifyAuthError('Invalid email or password').code, 'invalid_credentials')
    assert.strictEqual(classifyAuthError('Service unavailable (HTTP 503)').code, 'service_unavailable')
    assert.strictEqual(classifyAuthError('Forbidden secret key in config').code, 'security_violation')
    assert.strictEqual(classifyAuthError('Some completely unknown failure').code, 'unknown')
  })

  // 22. evaluateStoredSession immediately marks expired sessions on startup (F1.19)
  it('22. evaluateStoredSession returns expired status when stored timestamp is past', async () => {
    const mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    const nowSeconds = Math.floor(Date.now() / 1000)
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, 'tok_expired')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN, 'ref_expired')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT, String(nowSeconds - 500))
    window.sessionStorage.setItem(
      AUTH_STORAGE_KEYS.CLOUD_USER,
      JSON.stringify({ id: 'usr_past', email: 'past@example.com', role: 'owner' }),
    )

    const evaluated = await evaluateStoredSession(mockApi)
    assert.strictEqual(evaluated.status, 'expired')
    assert.strictEqual(evaluated.mode, 'online')
    assert.strictEqual(evaluated.refreshToken, 'ref_expired')
  })

  // 23. Single-Flight Concurrent Refresh Regression Test (F1.19 CodeRabbit & Mutex Safety)
  it('23. concurrent refresh triggers share single-flight promise and issue exactly ONE refresh API call', async () => {
    const mockApi = new MockAuthApiClient()
    mockApi.refreshDelayMs = 25 // Artificial delay to ensure overlap
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    const nowSeconds = Math.floor(Date.now() / 1000)
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, 'tok_expiring')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN, 'ref_single_flight')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT, String(nowSeconds + 60))
    window.sessionStorage.setItem(
      AUTH_STORAGE_KEYS.CLOUD_USER,
      JSON.stringify({ id: 'usr_sf', email: 'sf@example.com', role: 'owner' }),
    )

    const user = { id: 'usr_sf', email: 'sf@example.com', role: 'owner' }

    // Trigger 2 concurrent refresh calls simultaneously
    const [res1, res2] = await Promise.all([
      performSingleFlightRefresh('ref_single_flight', mockApi, user),
      performSingleFlightRefresh('ref_single_flight', mockApi, user),
    ])

    assert.ok(res1)
    assert.ok(res2)
    assert.strictEqual(res1.session.access_token, res2.session.access_token)
    assert.strictEqual(res1.session.refresh_token, res2.session.refresh_token)
    assert.strictEqual(mockApi.refreshCount, 1)

    // Subsequent call after completion starts a new single-flight
    const res3 = await performSingleFlightRefresh('ref_single_flight', mockApi, user)
    assert.ok(res3)
    assert.strictEqual(mockApi.refreshCount, 2)
  })

  // 24. Single-Flight Concurrent Refresh Error Propagation (F1.19)
  it('24. concurrent refresh triggers both reject when refresh fails with invalid_grant', async () => {
    const mockApi = new MockAuthApiClient()
    mockApi.refreshDelayMs = 25
    setAuthApi(mockApi)

    const user = { id: 'usr_err', email: 'err@example.com', role: 'owner' }

    const results = await Promise.allSettled([
      performSingleFlightRefresh('invalid_refresh', mockApi, user),
      performSingleFlightRefresh('invalid_refresh', mockApi, user),
    ])

    assert.strictEqual(results[0].status, 'rejected')
    assert.strictEqual(results[1].status, 'rejected')
    assert.strictEqual(mockApi.refreshCount, 1)
  })

  // 25. Transient network failure on expiring soon session leaves credentials intact in storage (F1.19)
  it('25. transient error on expiring soon token leaves credentials in storage', async () => {
    const mockApi = new MockAuthApiClient()
    mockApi.shouldFailWith = 'Network error: Unable to reach Supabase authentication service'
    setAuthApi(mockApi)
    window.sessionStorage.clear()

    const nowSeconds = Math.floor(Date.now() / 1000)
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, 'tok_still_valid')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN, 'ref_still_valid')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT, String(nowSeconds + 120)) // 2 min remaining
    window.sessionStorage.setItem(
      AUTH_STORAGE_KEYS.CLOUD_USER,
      JSON.stringify({ id: 'usr_valid', email: 'valid@example.com', role: 'owner' }),
    )

    const user = { id: 'usr_valid', email: 'valid@example.com', role: 'owner' }
    await assert.rejects(
      () => performSingleFlightRefresh('ref_still_valid', mockApi, user),
      (err: unknown) => {
        const typed = classifyAuthError(err)
        assert.strictEqual(typed.code, 'network_error')
        return true
      },
    )

    // Storage is preserved because failure was transient network error
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), 'tok_still_valid')
    assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN), 'ref_still_valid')
  })

  // 26. MockAuthApiClient comprehensive edge branches (F1.19)
  it('26. MockAuthApiClient handles empty refresh tokens, rate limits, and unclassified errors', async () => {
    const mockApi = new MockAuthApiClient()

    await assert.rejects(
      () => mockApi.refreshOnlineSession('   '),
      (err: unknown) => {
        const typed = classifyAuthError(err)
        assert.strictEqual(typed.code, 'validation_error')
        return true
      },
    )

    await assert.rejects(
      () => mockApi.refreshOnlineSession('rate_limited'),
      (err: unknown) => {
        const typed = classifyAuthError(err)
        assert.strictEqual(typed.code, 'rate_limit')
        return true
      },
    )

    await assert.rejects(
      () => mockApi.refreshOnlineSession('unknown_error'),
      (err: unknown) => {
        const typed = classifyAuthError(err)
        assert.strictEqual(typed.code, 'unknown')
        return true
      },
    )
  })

  // 27. classifyAuthError covers unconfigured, validation, and object payloads (F1.19)
  it('27. classifyAuthError handles unconfigured, validation, non-error objects, and primitives', () => {
    assert.strictEqual(classifyAuthError('Missing configuration for Supabase url').code, 'unconfigured')
    assert.strictEqual(classifyAuthError('Validation failed: payload invalid').code, 'validation_error')
    assert.strictEqual(classifyAuthError({ message: 'Rate limit hit' }).code, 'rate_limit')
    assert.strictEqual(classifyAuthError({ error_description: 'Invalid login credentials' }).code, 'invalid_credentials')
    assert.strictEqual(classifyAuthError({ error: 'Service unavailable' }).code, 'service_unavailable')
    assert.strictEqual(classifyAuthError(12345).code, 'unknown')
    assert.strictEqual(classifyAuthError(null).code, 'unknown')
  })
})
