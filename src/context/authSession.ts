// Pure TypeScript session evaluation and storage manager
// F1.13 — Authentication screens and session lifecycle & F1.14 — Local PIN and Lock screen & F1.19 — Supabase Auth adapter hardening

import type { AuthMode, AuthStatus, SignInInput, LocalSignInInput, OnlineSession } from '../types/auth'
import type { LoginResult } from '../types/session'
import type { AuthApiClient } from '../services/authApi'
export const AUTH_STORAGE_KEYS = {
  SESSION_ID: 'pos_global_session_id',
  AUTH_MODE: 'pos_global_auth_mode',
  CLOUD_TOKEN: 'pos_global_cloud_token',
  CLOUD_REFRESH_TOKEN: 'pos_global_cloud_refresh_token',
  CLOUD_EXPIRES_AT: 'pos_global_cloud_expires_at',
  CLOUD_USER: 'pos_global_cloud_user',
} as const

export const DEFAULT_AUTH_MODE = 'online' as const

export const DEFAULT_EXPIRY_THRESHOLD_SECONDS = 300 // 5 minutes proactive refresh window

export interface AuthenticatedUser {
  id: string
  username?: string | null
  email?: string | null
  full_name?: string | null
  role: string
  branch_id?: string | null
  organization_id?: string | null
}

export interface RestoredSessionData {
  status: AuthStatus
  user: AuthenticatedUser | null
  sessionId: string | null
  mode: AuthMode
  refreshToken?: string | null
  expiresAt?: number | null
}

/**
 * Checks if a token expires within the given threshold (defaults to 5 minutes).
 */
export function isTokenExpiringSoon(
  expiresAt: number | undefined | null,
  thresholdSeconds = DEFAULT_EXPIRY_THRESHOLD_SECONDS,
): boolean {
  if (expiresAt === undefined || expiresAt === null || Number.isNaN(expiresAt) || expiresAt <= 0) {
    return true
  }
  const nowSeconds = Math.floor(Date.now() / 1000)
  return expiresAt - nowSeconds <= thresholdSeconds
}

/**
 * Safely clears all stored authentication credentials and tokens from tab sessionStorage.
 */
export function clearStoredAuth(): void {
  if (typeof window !== 'undefined' && window.sessionStorage) {
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.SESSION_ID)
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN)
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN)
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT)
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.CLOUD_USER)
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.AUTH_MODE)
  }
}

/**
 * Persists active online session state to tab sessionStorage without retaining stale tokens or writing "null".
 */
export function storeOnlineSession(session: OnlineSession, user: AuthenticatedUser): void {
  if (typeof window !== 'undefined' && window.sessionStorage) {
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, session.access_token)

    if (
      session.refresh_token &&
      typeof session.refresh_token === 'string' &&
      session.refresh_token.trim().length > 0 &&
      session.refresh_token !== 'null' &&
      session.refresh_token !== 'undefined'
    ) {
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN, session.refresh_token.trim())
    } else {
      window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN)
    }

    let resolvedExpiresAt: number | null = null
    if (session.expires_at !== undefined && session.expires_at !== null && !Number.isNaN(session.expires_at)) {
      resolvedExpiresAt = session.expires_at
    } else if (
      session.expires_in !== undefined &&
      session.expires_in !== null &&
      !Number.isNaN(session.expires_in)
    ) {
      resolvedExpiresAt = Math.floor(Date.now() / 1000) + session.expires_in
    }

    if (resolvedExpiresAt !== null && !Number.isNaN(resolvedExpiresAt) && resolvedExpiresAt > 0) {
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT, String(resolvedExpiresAt))
    } else {
      window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT)
    }

    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.SESSION_ID, user.id)
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_USER, JSON.stringify(user))
  }
}

/**
 * Evaluates stored online session credentials, parsing expiration timestamps and enforcing fail-closed expired status.
 */
export async function restoreOnlineSession(
  token: string,
  rawUser: string | null,
  refreshToken?: string | null,
  expiresAtStr?: string | null,
): Promise<RestoredSessionData> {
  if (!token || !rawUser || token === 'null' || token === 'undefined') {
    clearStoredAuth()
    return { status: 'unauthenticated', user: null, sessionId: null, mode: 'online' }
  }

  try {
    const parsedUser = JSON.parse(rawUser) as AuthenticatedUser
    if (parsedUser?.id && parsedUser?.email) {
      const cleanExpiresAtStr =
        expiresAtStr && expiresAtStr !== 'null' && expiresAtStr !== 'undefined' ? expiresAtStr : null
      const parsedExpiresAt = cleanExpiresAtStr ? Number(cleanExpiresAtStr) : null
      const validExpiresAt =
        parsedExpiresAt !== null && !Number.isNaN(parsedExpiresAt) && parsedExpiresAt > 0
          ? parsedExpiresAt
          : null

      const cleanRefreshToken =
        refreshToken && refreshToken !== 'null' && refreshToken !== 'undefined'
          ? refreshToken.trim()
          : null

      const nowSeconds = Math.floor(Date.now() / 1000)
      if (validExpiresAt !== null && validExpiresAt <= nowSeconds) {
        return {
          status: 'expired',
          user: parsedUser,
          sessionId: parsedUser.id,
          mode: 'online',
          refreshToken: cleanRefreshToken,
          expiresAt: validExpiresAt,
        }
      }

      return {
        status: 'authenticated',
        user: parsedUser,
        sessionId: parsedUser.id,
        mode: 'online',
        refreshToken: cleanRefreshToken,
        expiresAt: validExpiresAt,
      }
    }
  } catch {
    // Fail closed on corrupt storage JSON
  }

  clearStoredAuth()
  return { status: 'expired', user: null, sessionId: null, mode: 'online' }
}

/**
 * Restores active local session from local SQLite backend.
 */
export async function restoreLocalSession(
  sessionId: string,
  apiClient: AuthApiClient,
): Promise<RestoredSessionData> {
  try {
    const state = await apiClient.getAuthState(sessionId)

    if (state.authenticated && state.session_id) {
      return {
        status: 'authenticated',
        user: {
          id: state.user_id || 'usr_local',
          username: state.user_id || 'operator',
          role: state.role || 'cashier',
          branch_id: state.branch_id || null,
          organization_id: state.organization_id || null,
        },
        sessionId: state.session_id,
        mode: 'local',
      }
    }
  } catch {
    // Fallthrough to expired
  }

  clearStoredAuth()
  return { status: 'expired', user: null, sessionId: null, mode: 'local' }
}

/**
 * Reads tab sessionStorage and restores active session according to stored authentication mode.
 */
export async function evaluateStoredSession(apiClient: AuthApiClient): Promise<RestoredSessionData> {
  if (typeof window === 'undefined' || !window.sessionStorage) {
    return { status: 'unauthenticated', user: null, sessionId: null, mode: DEFAULT_AUTH_MODE }
  }

  const storedMode = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE) as AuthMode | null
  const storedSessionId = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID)
  const storedCloudToken = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN)
  const storedCloudRefreshToken = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN)
  const storedCloudExpiresAt = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT)
  const storedCloudUser = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_USER)

  if (storedMode === 'online' && storedCloudToken) {
    return restoreOnlineSession(storedCloudToken, storedCloudUser, storedCloudRefreshToken, storedCloudExpiresAt)
  }

  if (storedSessionId) {
    return restoreLocalSession(storedSessionId, apiClient)
  }

  return { status: 'unauthenticated', user: null, sessionId: null, mode: DEFAULT_AUTH_MODE }
}

/**
 * Performs online authentication against Supabase Auth and persists active session tokens.
 */
export async function performOnlineLogin(
  credentials: SignInInput,
  apiClient: AuthApiClient,
): Promise<{ session: OnlineSession; user: AuthenticatedUser }> {
  const onlineSession = await apiClient.onlineLogin(credentials)
  const user: AuthenticatedUser = {
    id: onlineSession.user.id,
    email: onlineSession.user.email,
    full_name: onlineSession.user.email.split('@')[0],
    role: 'owner',
  }

  storeOnlineSession(onlineSession, user)
  return { session: onlineSession, user }
}

/**
 * Performs online token refresh against Supabase Auth, updating session storage atomically.
 */
export async function performTokenRefresh(
  refreshToken: string,
  apiClient: AuthApiClient,
  currentUser?: AuthenticatedUser | null,
): Promise<{ session: OnlineSession; user: AuthenticatedUser }> {
  const onlineSession = await apiClient.refreshOnlineSession(refreshToken)
  const resolvedId = onlineSession.user?.id || currentUser?.id || 'usr_online'
  const resolvedEmail = onlineSession.user?.email || currentUser?.email || ''
  const user: AuthenticatedUser = {
    id: resolvedId,
    email: resolvedEmail,
    full_name: resolvedEmail ? resolvedEmail.split('@')[0] : 'User',
    role: currentUser?.role || 'owner',
    branch_id: currentUser?.branch_id || null,
    organization_id: currentUser?.organization_id || null,
  }

  storeOnlineSession(onlineSession, user)
  return { session: onlineSession, user }
}

/**
 * Performs local username/password authentication against local SQLite backend.
 */
export async function performLocalLogin(
  credentials: LocalSignInInput,
  apiClient: AuthApiClient,
): Promise<{ result: LoginResult; user: AuthenticatedUser | null }> {
  const result = await apiClient.localLogin(credentials)

  if (result.success && result.session_id) {
    if (typeof window !== 'undefined' && window.sessionStorage) {
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'local')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.SESSION_ID, result.session_id)
    }
    const user: AuthenticatedUser = {
      id: result.user_id || 'usr_local',
      username: credentials.username,
      role: result.role || 'cashier',
      branch_id: result.branch_id || null,
    }
    return { result, user }
  }

  return { result, user: null }
}

/**
 * Verifies local POS operator PIN and restores terminal session.
 */
export async function performPinUnlock(
  userId: string,
  pin: string,
  apiClient: AuthApiClient,
  existingBranchId?: string | null,
): Promise<{ result: LoginResult; user: AuthenticatedUser | null }> {
  const result = await apiClient.verifyPin(userId, pin)

  if (result.success && result.session_id) {
    if (typeof window !== 'undefined' && window.sessionStorage) {
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'local')
      window.sessionStorage.setItem(AUTH_STORAGE_KEYS.SESSION_ID, result.session_id)
    }
    const resolvedBranchId = result.branch_id || existingBranchId || null
    const user: AuthenticatedUser = {
      id: result.user_id || userId,
      role: result.role || 'cashier',
      branch_id: resolvedBranchId,
    }
    return { result, user }
  }

  return { result, user: null }
}

async function revokeCloudSession(apiClient: AuthApiClient, cloudToken?: string | null): Promise<void> {
  const token =
    cloudToken ||
    (typeof window !== 'undefined' && window.sessionStorage
      ? window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN)
      : null)
  if (token) {
    try {
      await apiClient.onlineLogout(token)
    } catch {
      // Fail-closed without throwing uncaught rejection
    }
  }
}

async function revokeLocalSession(apiClient: AuthApiClient, sessionId: string): Promise<void> {
  try {
    await apiClient.logout(sessionId)
  } catch {
    // Fail-closed without throwing uncaught rejection
  }
}

/**
 * Performs complete session logout, invalidating cloud or local sessions and clearing storage.
 */
export async function performLogout(
  sessionId: string | null,
  authMode: AuthMode,
  apiClient?: AuthApiClient,
  cloudToken?: string | null,
): Promise<void> {
  if (apiClient) {
    if (authMode === 'online') {
      await revokeCloudSession(apiClient, cloudToken)
    } else if (sessionId) {
      await revokeLocalSession(apiClient, sessionId)
    }
  }
  clearStoredAuth()
}
