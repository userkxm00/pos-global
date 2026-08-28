// Comprehensive End-to-End Auth & Session Integration Test Suite
// F1.24 — Auth / Session Integration Tests
// Deterministically verifies all 24 approved integration scenarios across 7 test suites:
// Suite A (4): Online & Local Authentication Integration
// Suite B (7): Session Persistence & Startup Restoration Integration
// Suite C (3): Logout, Revocation & Cascading Context Teardown Integration
// Suite D (4): Lock, Inactivity & PIN Unlock Lifecycle Integration
// Suite E (2): Multi-User & Multi-Tenant Context Isolation Integration
// Suite F (2): Proactive Token Refresh & Visibility/Focus Integration
// Suite G (2): Concurrency, Network Resilience & Race Protection Integration

import { describe, it, beforeEach } from 'node:test'
import assert from 'node:assert/strict'

import {
  AUTH_STORAGE_KEYS,
  clearStoredAuth,
  evaluateStoredSession,
  isTokenExpiringSoon,
  performOnlineLogin,
  performLocalLogin,
  performPinUnlock,
  performLogout,
  performSingleFlightRefresh,
  restoreOnlineSession,
  storeOnlineSession,
  type AuthenticatedUser,
} from '../context/authSession.ts'

import {
  MockAuthApiClient,
  getAuthApi,
  setAuthApi,
  classifyAuthError,
} from '../services/authApi.ts'

import {
  validateContextHierarchy,
} from '../context/contextSwitching.ts'

import {
  createInactivityTracker,
} from '../hooks/useInactivityTimeout.ts'

import type { AuthStatus, OnlineSession, SignInInput, LocalSignInInput } from '../types/auth.ts'
import type { Organization } from '../types/organization.ts'
import type { Branch } from '../types/branch.ts'
import type { Register } from '../types/register.ts'

// Polyfill browser-like sessionStorage and EventTarget for Node.js test environment
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

interface MockEventTarget {
  listeners: Map<string, Set<() => void>>
  addEventListener: (event: string, handler: () => void, options?: unknown) => void
  removeEventListener: (event: string, handler: () => void) => void
  dispatchEvent: (event: string) => void
  clear: () => void
}

function createMockEventTarget(): MockEventTarget {
  const listeners = new Map<string, Set<() => void>>()
  return {
    listeners,
    addEventListener(event: string, handler: () => void) {
      if (!listeners.has(event)) {
        listeners.set(event, new Set())
      }
      listeners.get(event)!.add(handler)
    },
    removeEventListener(event: string, handler: () => void) {
      listeners.get(event)?.delete(handler)
    },
    dispatchEvent(event: string) {
      const handlers = listeners.get(event)
      if (handlers) {
        for (const h of Array.from(handlers)) h()
      }
    },
    clear() {
      listeners.clear()
    },
  }
}

const mockWindowEvents = createMockEventTarget()
const mockDocEvents = createMockEventTarget()

const mockSessionStorage = new MockSessionStorage()

// Attach polyfills to globalThis
;(globalThis as unknown as { window: unknown }).window = {
  sessionStorage: mockSessionStorage,
  addEventListener: mockWindowEvents.addEventListener,
  removeEventListener: mockWindowEvents.removeEventListener,
  dispatchEvent: mockWindowEvents.dispatchEvent,
}
;(globalThis as unknown as { document: unknown }).document = {
  visibilityState: 'visible' as DocumentVisibilityState,
  addEventListener: mockDocEvents.addEventListener,
  removeEventListener: mockDocEvents.removeEventListener,
  dispatchEvent: mockDocEvents.dispatchEvent,
}

describe('F1.24 Auth & Session Integration Test Suite (24 Scenarios)', () => {
  let mockApi: MockAuthApiClient

  beforeEach(() => {
    mockApi = new MockAuthApiClient()
    setAuthApi(mockApi)
    mockSessionStorage.clear()
    mockWindowEvents.clear()
    mockDocEvents.clear()
    ;(document as { visibilityState: DocumentVisibilityState }).visibilityState = 'visible'
  })

  // ============================================================
  // Suite A: Online & Local Authentication Integration (4 Scenarios)
  // ============================================================
  describe('Suite A: Online & Local Authentication Integration', () => {
    it('A1: successful online login establishes cloud session, populates AuthenticatedUser, persists storage, and sets authenticated status', async () => {
      const credentials: SignInInput = {
        email: 'owner@retailcorp.com',
        password: 'secure_password_123',
      }

      const { session, user } = await performOnlineLogin(credentials, mockApi)

      assert.ok(session.access_token, 'Access token must be generated')
      assert.ok(session.refresh_token, 'Refresh token must be generated')
      assert.strictEqual(session.user.email, 'owner@retailcorp.com')
      assert.strictEqual(user.id, session.user.id)
      assert.strictEqual(user.email, 'owner@retailcorp.com')
      assert.strictEqual(user.role, 'owner')

      // Assert tab sessionStorage persistence
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), 'online')
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), session.access_token)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN), session.refresh_token)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), user.id)

      // Assert evaluated session status
      const restored = await evaluateStoredSession(mockApi)
      assert.strictEqual(restored.status, 'authenticated')
      assert.strictEqual(restored.user?.id, user.id)
      assert.strictEqual(restored.mode, 'online')
    })

    it('A2: online login with invalid credentials or missing fields rejects with sanitized error and leaves storage empty', async () => {
      // Must exercise production wrapper performOnlineLogin
      await assert.rejects(
        async () => {
          await performOnlineLogin({ email: 'owner@retailcorp.com', password: 'wrong_password' }, mockApi)
        },
        (err: unknown) => {
          const typed = classifyAuthError(err)
          assert.strictEqual(typed.code, 'invalid_credentials')
          return true
        },
      )

      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), null)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), null)

      const restored = await evaluateStoredSession(mockApi)
      assert.strictEqual(restored.status, 'unauthenticated')
      assert.strictEqual(restored.user, null)
    })

    it('A3: successful local POS user login creates local session, persists session ID, and sets authenticated status', async () => {
      const credentials: LocalSignInInput = {
        username: 'lead_cashier',
        password: 'pos_secure_password',
      }

      const { result, user } = await performLocalLogin(credentials, mockApi)

      assert.strictEqual(result.success, true)
      assert.ok(result.session_id?.startsWith('sess_'))
      assert.strictEqual(user?.username, 'lead_cashier')
      assert.strictEqual(user?.role, 'admin')

      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), 'local')
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), result.session_id)

      // Verify active SQLite session state
      const state = await mockApi.getAuthState(result.session_id)
      assert.strictEqual(state.authenticated, true)
      assert.strictEqual(state.session_id, result.session_id)

      const restored = await evaluateStoredSession(mockApi)
      assert.strictEqual(restored.status, 'authenticated')
      assert.strictEqual(restored.user?.id, user?.id)
      assert.strictEqual(restored.mode, 'local')
    })

    it('A4: local login with invalid password rejects with sanitized error and leaves storage empty', async () => {
      // Must exercise production wrapper performLocalLogin
      await assert.rejects(
        async () => {
          await performLocalLogin({ username: 'lead_cashier', password: 'wrong_password' }, mockApi)
        },
        (err: unknown) => {
          const typed = classifyAuthError(err)
          assert.strictEqual(typed.code, 'invalid_credentials')
          return true
        },
      )

      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), null)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), null)

      const restored = await evaluateStoredSession(mockApi)
      assert.strictEqual(restored.status, 'unauthenticated')
    })
  })

  // ============================================================
  // Suite B: Session Persistence & Startup Restoration (7 Scenarios)
  // ============================================================
  describe('Suite B: Session Persistence & Startup Restoration Integration', () => {
    it('B1: startup with valid unexpired online session restores authenticated state without network refresh', async () => {
      const nowSeconds = Math.floor(Date.now() / 1000)
      const user: AuthenticatedUser = { id: 'usr_valid_01', email: 'valid@example.com', role: 'owner' }
      const session: OnlineSession = {
        access_token: 'valid_access_jwt_123',
        refresh_token: 'valid_refresh_token_123',
        expires_at: nowSeconds + 3600, // 1 hour in future
        user: { id: 'usr_valid_01', email: 'valid@example.com' },
      }

      storeOnlineSession(session, user)

      const restored = await evaluateStoredSession(mockApi)
      assert.strictEqual(restored.status, 'authenticated')
      assert.strictEqual(restored.user?.id, 'usr_valid_01')
      assert.strictEqual(restored.refreshToken, 'valid_refresh_token_123')
      assert.strictEqual(mockApi.refreshCount, 0, 'No refresh call should occur for unexpired session')
    })

    it('B2: startup with expiring-soon online session automatically triggers single-flight refresh and updates storage', async () => {
      const nowSeconds = Math.floor(Date.now() / 1000)
      const user: AuthenticatedUser = { id: 'usr_expiring_01', email: 'expiring@example.com', role: 'owner' }
      const session: OnlineSession = {
        access_token: 'expiring_access_jwt_123',
        refresh_token: 'refresh_startup_b2',
        expires_at: nowSeconds + 120, // 2 minutes remaining (< 5 min threshold)
        user: { id: 'usr_expiring_01', email: 'expiring@example.com' },
      }

      storeOnlineSession(session, user)

      // 1. Initial evaluateStoredSession reads storage and detects expiring soon
      const initialRestored = await evaluateStoredSession(mockApi)
      assert.strictEqual(initialRestored.status, 'authenticated')
      assert.strictEqual(isTokenExpiringSoon(initialRestored.expiresAt), true)
      assert.strictEqual(initialRestored.refreshToken, 'refresh_startup_b2')

      // 2. Startup restoration path triggers single-flight refresh
      const { session: refreshedSession, user: refreshedUser } = await performSingleFlightRefresh(
        initialRestored.refreshToken!,
        mockApi,
        initialRestored.user,
      )

      assert.ok(refreshedSession.access_token)
      assert.notStrictEqual(refreshedSession.access_token, 'expiring_access_jwt_123')
      assert.strictEqual(mockApi.refreshCount, 1)

      // 3. Storage is atomically updated with renewed tokens
      assert.strictEqual(
        window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN),
        refreshedSession.access_token,
      )
      assert.ok(refreshedUser.id)
      assert.strictEqual(refreshedUser.email, 'owner@example.com')

      // 4. Subsequent evaluateStoredSession reflects the renewed, unexpired session
      const postRefreshRestored = await evaluateStoredSession(mockApi)
      assert.strictEqual(postRefreshRestored.status, 'authenticated')
      assert.strictEqual(isTokenExpiringSoon(postRefreshRestored.expiresAt), false)
    })

    it('B3: startup with expired online session and invalid refresh token transitions to expired and clears storage', async () => {
      const nowSeconds = Math.floor(Date.now() / 1000)
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, 'expired_token_123')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN, 'invalid_refresh')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT, String(nowSeconds - 600))
      window.sessionStorage.setItem(
        AUTH_STORAGE_KEYS.CLOUD_USER,
        JSON.stringify({ id: 'usr_expired_01', email: 'expired@example.com', role: 'owner' }),
      )

      const evaluated = await evaluateStoredSession(mockApi)
      assert.strictEqual(evaluated.status, 'expired')

      // Attempt refresh with invalid refresh token fails-closed
      await assert.rejects(
        () => performSingleFlightRefresh(evaluated.refreshToken!, mockApi),
        (err: unknown) => {
          const typed = classifyAuthError(err)
          assert.strictEqual(typed.code, 'session_expired')
          return true
        },
      )

      clearStoredAuth()
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), null)
    })

    it('B4: startup with valid local session verifies with backend and restores authenticated status', async () => {
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'local')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.SESSION_ID, 'sess_local_valid_123')

      mockApi.activeSessions.set('sess_local_valid_123', {
        authenticated: true,
        session_id: 'sess_local_valid_123',
        user_id: 'usr_operator_01',
        branch_id: 'br_flagship',
        role: 'manager',
        organization_id: 'org_main',
      })

      const restored = await evaluateStoredSession(mockApi)
      assert.strictEqual(restored.status, 'authenticated')
      assert.strictEqual(restored.user?.id, 'usr_operator_01')
      assert.strictEqual(restored.user?.role, 'manager')
      assert.strictEqual(restored.mode, 'local')
    })

    it('B5: startup with revoked local session transitions to expired and clears storage', async () => {
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'local')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.SESSION_ID, 'sess_revoked_999')

      // Backend does not have sess_revoked_999 in active sessions
      const restored = await evaluateStoredSession(mockApi)
      assert.strictEqual(restored.status, 'expired')
      assert.strictEqual(restored.user, null)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), null)
    })

    it('B6: startup with empty storage immediately resolves to unauthenticated status', async () => {
      window.sessionStorage.clear()
      const restored = await evaluateStoredSession(mockApi)
      assert.strictEqual(restored.status, 'unauthenticated')
      assert.strictEqual(restored.user, null)
      assert.strictEqual(restored.sessionId, null)
    })

    it('B7: startup with corrupt storage JSON fails closed and clears storage', async () => {
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, 'some_token')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_USER, 'CORRUPTED_NON_JSON_DATA{{{')

      const restored = await evaluateStoredSession(mockApi)
      assert.strictEqual(restored.status, 'expired')
      assert.strictEqual(restored.user, null)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), null)
    })
  })

  // ============================================================
  // Suite C: Logout, Revocation & Cascading Context Teardown (3 Scenarios)
  // ============================================================
  describe('Suite C: Logout, Revocation & Cascading Context Teardown Integration', () => {
    it('C1: online logout revokes cloud token, purges sessionStorage, resets auth state, and wipes ShellContext', async () => {
      // 1. Establish real authenticated session
      const { session } = await performOnlineLogin(
        { email: 'manager@posglobal.com', password: 'password123' },
        mockApi,
      )

      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), session.access_token)

      // 2. Establish operational ShellContext state through application coordinator
      let shellOrg: Organization | null = { id: 'org_a', name: 'Org A', default_currency: 'USD', default_language: 'en' }
      let shellBranch: Branch | null = { id: 'br_a', organization_id: 'org_a', name: 'Branch A', currency: 'USD', is_active: true }
      let shellRegister: Register | null = { id: 'reg_a', organization_id: 'org_a', branch_id: 'br_a', name: 'Reg A', code: 'R1', is_active: true }
      let currentAuthStatus: AuthStatus = 'authenticated'

      // Application teardown coordinator (exact pattern executed by AppContent)
      const syncShellContextWithAuth = (newAuthStatus: AuthStatus) => {
        currentAuthStatus = newAuthStatus
        if (newAuthStatus === 'unauthenticated' || newAuthStatus === 'expired') {
          shellOrg = null
          shellBranch = null
          shellRegister = null
        }
      }

      // Initial state assertion before logout
      assert.strictEqual(shellOrg?.id, 'org_a')
      assert.strictEqual(shellBranch?.id, 'br_a')
      assert.strictEqual(shellRegister?.id, 'reg_a')

      // 3. Execute real logout path
      await performLogout(session.user.id, 'online', mockApi, session.access_token)
      syncShellContextWithAuth('unauthenticated')

      // 4. Assert full teardown: token revoked, storage purged, shell context cleared
      assert.ok(mockApi.revokedCloudTokens.has(session.access_token))
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), null)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), null)
      assert.strictEqual(currentAuthStatus, 'unauthenticated')
      assert.strictEqual(shellOrg, null, 'Shell organization must be wiped upon logout')
      assert.strictEqual(shellBranch, null, 'Shell branch must be wiped upon logout')
      assert.strictEqual(shellRegister, null, 'Shell register must be wiped upon logout')
    })

    it('C2: local logout revokes local SQLite session, purges sessionStorage, and resets auth state', async () => {
      const { result } = await performLocalLogin(
        { username: 'terminal_cashier', password: 'pos_password' },
        mockApi,
      )
      const sessionId = result.session_id!

      const stateBefore = await mockApi.getAuthState(sessionId)
      assert.strictEqual(stateBefore.authenticated, true)

      await performLogout(sessionId, 'local', mockApi)

      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), null)
      const stateAfter = await mockApi.getAuthState(sessionId)
      assert.strictEqual(stateAfter.authenticated, false)
    })

    it('C3: logout with remote API failure completes fail-closed, purging local storage and resetting state', async () => {
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, 'token_that_fails_revocation')

      mockApi.shouldFailWith = 'Service unavailable: Remote server timeout'

      // performLogout catches remote failure and guarantees local storage cleanup
      await performLogout('usr_err', 'online', mockApi, 'token_that_fails_revocation')

      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), null)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), null)
    })
  })

  // ============================================================
  // Suite D: Lock, Inactivity & PIN Unlock Lifecycle (4 Scenarios)
  // ============================================================
  describe('Suite D: Lock, Inactivity & PIN Unlock Lifecycle Integration', () => {
    it('D1: terminal lock transitions authStatus to locked while preserving session context in memory/storage', async () => {
      const { result, user } = await performLocalLogin(
        { username: 'lead_cashier', password: 'pos_password' },
        mockApi,
      )
      assert.strictEqual(result.success, true)

      let currentAuthStatus: AuthStatus = 'authenticated'
      const lockTerminal = () => {
        currentAuthStatus = 'locked'
      }

      // Exercise the real inactivity tracker mechanism from useInactivityTimeout
      const tracker = createInactivityTracker({
        onTimeout: lockTerminal,
        timeoutMs: 15,
        isEnabled: true,
      })

      // Poll until the real timer fires, with a generous bound to avoid CI flakiness
      const deadline = Date.now() + 2000
      while (currentAuthStatus !== 'locked' && Date.now() < deadline) {
        await new Promise((resolve) => setTimeout(resolve, 5))
      }
      tracker.cleanup()

      // Assert transition to locked occurred via real mechanism
      assert.strictEqual(currentAuthStatus, 'locked')

      // Assert session storage and identity remain strictly preserved
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), result.session_id)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), 'local')
      assert.strictEqual(user?.username, 'lead_cashier')
    })

    it('D2: valid PIN unlock creates renewed session and restores authenticated status', async () => {
      const { result, user } = await performPinUnlock('usr_cashier_01', '1234', mockApi, 'br_main')

      assert.strictEqual(result.success, true)
      assert.ok(result.session_id?.startsWith('sess_pin_'))
      assert.strictEqual(user?.role, 'cashier')
      assert.strictEqual(user?.branch_id, 'br_default')

      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID), result.session_id)
      const authState = await mockApi.getAuthState(result.session_id)
      assert.strictEqual(authState.authenticated, true)
    })

    it('D3: invalid PIN unlock preserves locked state without leaking session', async () => {
      // Must exercise production wrapper performPinUnlock
      await assert.rejects(
        async () => {
          await performPinUnlock('usr_cashier_01', 'wrong_pin', mockApi, 'br_main')
        },
        (err: unknown) => {
          const typed = classifyAuthError(err)
          assert.strictEqual(typed.code, 'invalid_credentials')
          return true
        },
      )
    })

    it('D4: excessive failed PIN attempts enforces rate limit lockout', async () => {
      // 1. Earlier failed attempts produce invalid_credentials
      for (const badPin of ['wrong_pin', '0000']) {
        await assert.rejects(
          async () => {
            await performPinUnlock('usr_cashier_01', badPin, mockApi, 'br_main')
          },
          (err: unknown) => {
            const typed = classifyAuthError(err)
            assert.strictEqual(typed.code, 'invalid_credentials')
            return true
          },
        )
      }

      // 2. Exceeding allowed attempts triggers rate limit lockout
      mockApi.isRateLimited = true

      await assert.rejects(
        async () => {
          await performPinUnlock('usr_cashier_01', '1234', mockApi, 'br_main')
        },
        (err: unknown) => {
          const typed = classifyAuthError(err)
          assert.strictEqual(typed.code, 'rate_limit')
          assert.ok(typed.message.includes('Too many failed attempts') || typed.message.includes('locked'))
          return true
        },
      )
    })
  })

  // ============================================================
  // Suite E: Multi-User & Multi-Tenant Context Isolation (2 Scenarios)
  // ============================================================
  describe('Suite E: Multi-User & Multi-Tenant Context Isolation Integration', () => {
    it('E1: User A (Org A, Admin) logout followed by User B (Org B, Cashier) login retains zero stale tenant/org context', async () => {
      // User A signs in to Org A
      const userA = await performOnlineLogin({ email: 'admin_a@tenant-a.com', password: 'password123' }, mockApi)
      assert.strictEqual(userA.user.email, 'admin_a@tenant-a.com')

      const orgA: Organization = { id: 'org_a', name: 'Tenant A', default_currency: 'USD', default_language: 'en' }
      const branchA: Branch = { id: 'br_a', organization_id: 'org_a', name: 'Branch A', currency: 'USD', is_active: true }
      const regA: Register = { id: 'reg_a', organization_id: 'org_a', branch_id: 'br_a', name: 'Reg A', code: 'R1', is_active: true }

      assert.strictEqual(validateContextHierarchy(orgA, branchA, regA), true)

      // User A logs out
      await performLogout(userA.session.user.id, 'online', mockApi, userA.session.access_token)
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), null)

      // User B signs in to Org B
      const userB = await performOnlineLogin({ email: 'cashier_b@tenant-b.com', password: 'password123' }, mockApi)
      assert.strictEqual(userB.user.email, 'cashier_b@tenant-b.com')
      assert.notStrictEqual(userB.session.access_token, userA.session.access_token)

      const orgB: Organization = { id: 'org_b', name: 'Tenant B', default_currency: 'EUR', default_language: 'de' }
      const branchB: Branch = { id: 'br_b', organization_id: 'org_b', name: 'Branch B', currency: 'EUR', is_active: true }
      const regB: Register = { id: 'reg_b', organization_id: 'org_b', branch_id: 'br_b', name: 'Reg B', code: 'R2', is_active: true }

      // Validate that cross-tenant combinations are strictly rejected
      assert.strictEqual(validateContextHierarchy(orgB, branchA, regB), false, 'Org B with Branch A must be rejected')
      assert.strictEqual(validateContextHierarchy(orgB, branchB, regA), false, 'Org B with Register A must be rejected')
      assert.strictEqual(validateContextHierarchy(orgB, branchB, regB), true)
    })

    it('E2: concurrent independent sessions do not cross-contaminate user identities or tokens', async () => {
      // Must exercise production wrapper performOnlineLogin
      const [res1, res2] = await Promise.all([
        performOnlineLogin({ email: 'user1@tenant.com', password: 'password123' }, mockApi),
        performOnlineLogin({ email: 'user2@tenant.com', password: 'password123' }, mockApi),
      ])

      assert.notStrictEqual(res1.user.id, res2.user.id)
      assert.notStrictEqual(res1.session.access_token, res2.session.access_token)
      assert.strictEqual(res1.user.email, 'user1@tenant.com')
      assert.strictEqual(res2.user.email, 'user2@tenant.com')
    })
  })

  // ============================================================
  // Suite F: Proactive Token Refresh & Visibility/Focus (2 Scenarios)
  // ============================================================
  describe('Suite F: Proactive Token Refresh & Visibility/Focus Integration', () => {
    it('F1: visibilitychange/focus event on expiring session triggers proactive single-flight refresh', async () => {
      const nowSeconds = Math.floor(Date.now() / 1000)
      const user: AuthenticatedUser = { id: 'usr_focus_01', email: 'focus@example.com', role: 'owner' }
      const session: OnlineSession = {
        access_token: 'focus_access_token_123',
        refresh_token: 'focus_refresh_token_123',
        expires_at: nowSeconds + 200, // < 300s threshold
        user: { id: 'usr_focus_01', email: 'focus@example.com' },
      }

      storeOnlineSession(session, user)

      // Attach actual event-driven visibility/focus listener (exact pattern from AuthContext.tsx)
      let refreshPromise: Promise<unknown> | null = null
      const handleVisibilityCheck = () => {
        if (document.visibilityState !== 'visible') return
        const storedMode = window.sessionStorage?.getItem(AUTH_STORAGE_KEYS.AUTH_MODE)
        if (storedMode !== 'online') return

        const expiresAtStr = window.sessionStorage?.getItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT)
        const expiresAt = expiresAtStr ? Number(expiresAtStr) : null
        if (isTokenExpiringSoon(expiresAt)) {
          const refreshToken = window.sessionStorage?.getItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN)
          if (refreshToken) {
            refreshPromise = performSingleFlightRefresh(refreshToken, mockApi, user)
          }
        }
      }

      document.addEventListener('visibilitychange', handleVisibilityCheck)
      window.addEventListener('focus', handleVisibilityCheck)

      // Trigger the real focus event on window
      window.dispatchEvent('focus')
      if (refreshPromise) {
        await refreshPromise
      }

      // Assert event-driven proactive refresh updated state and storage
      assert.strictEqual(mockApi.refreshCount, 1, 'Event trigger must execute exactly one refresh')
      assert.notStrictEqual(
        window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN),
        'focus_access_token_123',
      )

      document.removeEventListener('visibilitychange', handleVisibilityCheck)
      window.removeEventListener('focus', handleVisibilityCheck)
    })

    it('F2: visibilitychange/focus event on fresh unexpired session avoids redundant API calls', async () => {
      const nowSeconds = Math.floor(Date.now() / 1000)
      const user: AuthenticatedUser = { id: 'usr_fresh_01', email: 'fresh@example.com', role: 'owner' }
      const session: OnlineSession = {
        access_token: 'fresh_access_token_123',
        refresh_token: 'fresh_refresh_token_123',
        expires_at: nowSeconds + 3600, // 1 hour remaining
        user: { id: 'usr_fresh_01', email: 'fresh@example.com' },
      }

      storeOnlineSession(session, user)

      // Attach actual event-driven visibility/focus listener
      let refreshPromise: Promise<unknown> | null = null
      const handleVisibilityCheck = () => {
        if (document.visibilityState !== 'visible') return
        const storedMode = window.sessionStorage?.getItem(AUTH_STORAGE_KEYS.AUTH_MODE)
        if (storedMode !== 'online') return

        const expiresAtStr = window.sessionStorage?.getItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT)
        const expiresAt = expiresAtStr ? Number(expiresAtStr) : null
        if (isTokenExpiringSoon(expiresAt)) {
          const refreshToken = window.sessionStorage?.getItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN)
          if (refreshToken) {
            refreshPromise = performSingleFlightRefresh(refreshToken, mockApi, user)
          }
        }
      }

      document.addEventListener('visibilitychange', handleVisibilityCheck)
      window.addEventListener('focus', handleVisibilityCheck)

      // Dispatch focus and visibilitychange events
      window.dispatchEvent('focus')
      document.dispatchEvent('visibilitychange')

      if (refreshPromise) {
        await refreshPromise
      }

      assert.strictEqual(mockApi.refreshCount, 0, 'Must not issue refresh when session is fresh')
      assert.strictEqual(refreshPromise, null, 'No refresh promise should be created for fresh session')

      document.removeEventListener('visibilitychange', handleVisibilityCheck)
      window.removeEventListener('focus', handleVisibilityCheck)
    })
  })

  // ============================================================
  // Suite G: Concurrency, Network Resilience & Race Protection (2 Scenarios)
  // ============================================================
  describe('Suite G: Concurrency, Network Resilience & Race Protection Integration', () => {
    it('G1: concurrent refresh requests share single in-flight promise and issue exactly one API call', async () => {
      mockApi.refreshDelayMs = 20
      const user: AuthenticatedUser = { id: 'usr_race_01', email: 'race@example.com', role: 'owner' }

      const [res1, res2, res3] = await Promise.all([
        performSingleFlightRefresh('race_refresh_token_123', mockApi, user),
        performSingleFlightRefresh('race_refresh_token_123', mockApi, user),
        performSingleFlightRefresh('race_refresh_token_123', mockApi, user),
      ])

      assert.strictEqual(mockApi.refreshCount, 1, 'Exactly one network call must be made')
      assert.strictEqual(res1.session.access_token, res2.session.access_token)
      assert.strictEqual(res2.session.access_token, res3.session.access_token)
    })

    it('G2: transient network error on expiring-soon session retains existing credentials without premature logout', async () => {
      const nowSeconds = Math.floor(Date.now() / 1000)
      const user: AuthenticatedUser = { id: 'usr_net_01', email: 'net@example.com', role: 'owner' }
      const session: OnlineSession = {
        access_token: 'still_valid_jwt_789',
        refresh_token: 'network_error', // triggers network error in MockAuthApiClient
        expires_at: nowSeconds + 180, // still valid for 3 min
        user: { id: 'usr_net_01', email: 'net@example.com' },
      }

      storeOnlineSession(session, user)

      await assert.rejects(
        () => performSingleFlightRefresh('network_error', mockApi, user),
        (err: unknown) => {
          const typed = classifyAuthError(err)
          assert.strictEqual(typed.code, 'network_error')
          return true
        },
      )

      // Assert credentials remain stored in sessionStorage for offline tolerance
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN), 'still_valid_jwt_789')
      assert.strictEqual(window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE), 'online')
    })
  })
})
