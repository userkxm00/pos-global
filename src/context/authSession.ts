// Pure TypeScript session evaluation and storage manager
// F1.13 — Authentication screens and session lifecycle & F1.14 — Local PIN and Lock screen

import type { AuthMode, AuthStatus, SignInInput, LocalSignInInput, OnlineSession } from '../types/auth'
import type { LoginResult } from '../types/session'
import type { AuthApiClient } from '../services/authApi'

export const AUTH_STORAGE_KEYS = {
  SESSION_ID: 'pos_global_session_id',
  AUTH_MODE: 'pos_global_auth_mode',
  CLOUD_TOKEN: 'pos_global_cloud_token',
  CLOUD_USER: 'pos_global_cloud_user',
} as const

export const DEFAULT_AUTH_MODE = 'online' as const

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
}

export function clearStoredAuth(): void {
  if (typeof window !== 'undefined' && window.sessionStorage) {
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.SESSION_ID)
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN)
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.CLOUD_USER)
    window.sessionStorage.removeItem(AUTH_STORAGE_KEYS.AUTH_MODE)
  }
}

export async function restoreOnlineSession(token: string, rawUser: string | null): Promise<RestoredSessionData> {
  if (!token || !rawUser) {
    clearStoredAuth()
    return { status: 'unauthenticated', user: null, sessionId: null, mode: 'online' }
  }

  try {
    const parsedUser = JSON.parse(rawUser) as AuthenticatedUser
    if (parsedUser?.id && parsedUser?.email) {
      return {
        status: 'authenticated',
        user: parsedUser,
        sessionId: parsedUser.id,
        mode: 'online',
      }
    }
  } catch {
    // Fail closed on corrupt storage JSON
  }

  clearStoredAuth()
  return { status: 'expired', user: null, sessionId: null, mode: 'online' }
}

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

export async function evaluateStoredSession(apiClient: AuthApiClient): Promise<RestoredSessionData> {
  if (typeof window === 'undefined' || !window.sessionStorage) {
    return { status: 'unauthenticated', user: null, sessionId: null, mode: DEFAULT_AUTH_MODE }
  }

  const storedMode = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE) as AuthMode | null
  const storedSessionId = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.SESSION_ID)
  const storedCloudToken = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN)
  const storedCloudUser = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_USER)

  if (storedMode === 'online' && storedCloudToken) {
    return restoreOnlineSession(storedCloudToken, storedCloudUser)
  }

  if (storedSessionId) {
    return restoreLocalSession(storedSessionId, apiClient)
  }

  return { status: 'unauthenticated', user: null, sessionId: null, mode: DEFAULT_AUTH_MODE }
}

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

  if (typeof window !== 'undefined' && window.sessionStorage) {
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.AUTH_MODE, 'online')
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN, onlineSession.access_token)
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.SESSION_ID, onlineSession.user.id)
    window.sessionStorage.setItem(AUTH_STORAGE_KEYS.CLOUD_USER, JSON.stringify(user))
  }

  return { session: onlineSession, user }
}

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
      // Fail-closed
    }
  }
}

async function revokeLocalSession(apiClient: AuthApiClient, sessionId: string): Promise<void> {
  try {
    await apiClient.logout(sessionId)
  } catch {
    // Fail-closed
  }
}

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
