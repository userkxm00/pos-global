// AuthContext managing authentication lifecycle, session restoration, lock/unlock, logout, and token refresh
// F1.13 — Authentication screens and session lifecycle & F1.14 — Local PIN and Lock screen & F1.19 — Supabase Auth adapter hardening

import React, { createContext, useContext, useEffect, useState, useMemo, useCallback, useRef } from 'react'
import type { AuthMode, AuthStatus, SignInInput, LocalSignInInput, OnlineSession } from '../types/auth'
import type { LoginResult } from '../types/session'
import { getAuthApi, classifyAuthError } from '../services/authApi'
import {
  AUTH_STORAGE_KEYS,
  DEFAULT_AUTH_MODE,
  AuthenticatedUser,
  clearStoredAuth,
  evaluateStoredSession,
  isTokenExpiringSoon,
  performOnlineLogin,
  performSingleFlightRefresh,
  performLocalLogin,
  performPinUnlock,
  performLogout,
} from './authSession'

export type { AuthenticatedUser }

export interface AuthContextType {
  authStatus: AuthStatus
  authMode: AuthMode
  setAuthMode: (mode: AuthMode) => void
  activeUser: AuthenticatedUser | null
  sessionId: string | null
  isAuthenticating: boolean
  loginOnline: (credentials: SignInInput) => Promise<OnlineSession>
  refreshSession: () => Promise<OnlineSession | null>
  loginLocal: (credentials: LocalSignInInput) => Promise<LoginResult>
  lock: () => void
  unlockWithPin: (pin: string) => Promise<LoginResult>
  logout: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | null>(null)

export interface AuthProviderProps {
  children: React.ReactNode
  initialStatus?: AuthStatus
  initialUser?: AuthenticatedUser | null
  initialSessionId?: string | null
  initialMode?: AuthMode
}

interface RestoredOnlineState {
  status: AuthStatus
  user: AuthenticatedUser | null
  sessionId: string | null
  mode: AuthMode
}

async function handleStartupOnlineRefresh(
  restored: Awaited<ReturnType<typeof evaluateStoredSession>>,
  refreshFn: () => Promise<OnlineSession | null>,
): Promise<RestoredOnlineState | null> {
  if (!restored.refreshToken) {
    if (restored.status === 'expired') {
      clearStoredAuth()
      return { status: 'expired', user: null, sessionId: null, mode: 'online' }
    }
    return null
  }

  const refreshedSession = await refreshFn()
  if (refreshedSession) {
    return null
  }

  // If refresh failed, check if session was merely expiring soon and still in storage (transient error)
  const tokenStillStored =
    typeof window !== 'undefined' &&
    window.sessionStorage?.getItem(AUTH_STORAGE_KEYS.CLOUD_TOKEN)
  if (restored.status !== 'expired' && tokenStillStored && restored.user && restored.sessionId) {
    return { status: 'authenticated', user: restored.user, sessionId: restored.sessionId, mode: 'online' }
  }

  // Definitive auth failure -> fail closed
  clearStoredAuth()
  return { status: 'expired', user: null, sessionId: null, mode: 'online' }
}

export const AuthProvider: React.FC<AuthProviderProps> = ({
  children,
  initialStatus = 'authenticating',
  initialUser = null,
  initialSessionId = null,
  initialMode = DEFAULT_AUTH_MODE,
}) => {
  const [authStatus, setAuthStatus] = useState<AuthStatus>(initialStatus)
  const [authMode, setAuthMode] = useState<AuthMode>(initialMode)
  const [activeUser, setActiveUser] = useState<AuthenticatedUser | null>(initialUser)
  const [sessionId, setSessionId] = useState<string | null>(initialSessionId)
  const [isAuthenticating, setIsAuthenticating] = useState<boolean>(false)

  // Stable ref for activeUser to decouple callback identity from auth state updates
  const activeUserRef = useRef<AuthenticatedUser | null>(activeUser)
  useEffect(() => {
    activeUserRef.current = activeUser
  }, [activeUser])

  // Single-flight promise ref ensuring unified concurrency guard across startup, focus, and visibility events
  const inFlightRefreshPromiseRef = useRef<Promise<OnlineSession | null> | null>(null)

  const refreshSession = useCallback(async (): Promise<OnlineSession | null> => {
    if (inFlightRefreshPromiseRef.current) {
      return inFlightRefreshPromiseRef.current
    }
    if (typeof window === 'undefined' || !window.sessionStorage) return null

    const storedMode = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE)
    const storedRefreshToken = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN)

    if (storedMode !== 'online' || !storedRefreshToken) {
      return null
    }

    const refreshPromise = (async () => {
      try {
        const api = getAuthApi()
        const { session, user } = await performSingleFlightRefresh(
          storedRefreshToken,
          api,
          activeUserRef.current,
        )
        setActiveUser(user)
        setSessionId(user.id)
        setAuthMode('online')
        setAuthStatus('authenticated')
        return session
      } catch (err: unknown) {
        const typedErr = classifyAuthError(err)
        if (typedErr.code === 'session_expired' || typedErr.code === 'invalid_credentials') {
          clearStoredAuth()
          setAuthStatus('expired')
          setActiveUser(null)
          setSessionId(null)
        }
        return null
      } finally {
        inFlightRefreshPromiseRef.current = null
      }
    })()

    inFlightRefreshPromiseRef.current = refreshPromise
    return refreshPromise
  }, [])

  // Restore session on startup with immediate expiry evaluation and unified single-flight refresh
  useEffect(() => {
    let isMounted = true

    async function initSession() {
      if (initialUser && initialSessionId) {
        if (isMounted) setAuthStatus('authenticated')
        return
      }

      const api = getAuthApi()
      const restored = await evaluateStoredSession(api)
      if (!isMounted) return

      const needsImmediateRefresh =
        restored.mode === 'online' &&
        restored.user &&
        restored.sessionId &&
        (restored.status === 'expired' || isTokenExpiringSoon(restored.expiresAt))

      if (needsImmediateRefresh) {
        const nextState = await handleStartupOnlineRefresh(restored, refreshSession)
        if (!isMounted || !nextState) return
        setAuthStatus(nextState.status)
        setActiveUser(nextState.user)
        setSessionId(nextState.sessionId)
        setAuthMode(nextState.mode)
        return
      }

      if (isMounted) {
        setAuthStatus(restored.status)
        setActiveUser(restored.user)
        setSessionId(restored.sessionId)
        setAuthMode(restored.mode)
      }
    }

    void initSession()

    return () => {
      isMounted = false
    }
  }, [initialUser, initialSessionId, refreshSession])

  // Proactive token refresh check on window focus / visibility change
  useEffect(() => {
    if (typeof window === 'undefined') return

    const handleVisibilityCheck = () => {
      if (document.visibilityState !== 'visible') return
      if (authStatus !== 'authenticated' || authMode !== 'online') return

      const expiresAtStr = window.sessionStorage?.getItem(AUTH_STORAGE_KEYS.CLOUD_EXPIRES_AT)
      const expiresAt = expiresAtStr ? Number(expiresAtStr) : null
      if (isTokenExpiringSoon(expiresAt)) {
        void refreshSession()
      }
    }

    handleVisibilityCheck()
    document.addEventListener('visibilitychange', handleVisibilityCheck)
    window.addEventListener('focus', handleVisibilityCheck)

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityCheck)
      window.removeEventListener('focus', handleVisibilityCheck)
    }
  }, [authStatus, authMode, refreshSession])

  const loginOnline = useCallback(async (credentials: SignInInput): Promise<OnlineSession> => {
    setIsAuthenticating(true)
    try {
      const api = getAuthApi()
      const { session, user } = await performOnlineLogin(credentials, api)
      setActiveUser(user)
      setSessionId(session.user.id)
      setAuthMode('online')
      setAuthStatus('authenticated')
      return session
    } finally {
      setIsAuthenticating(false)
    }
  }, [])

  const loginLocal = useCallback(async (credentials: LocalSignInInput): Promise<LoginResult> => {
    setIsAuthenticating(true)
    try {
      const api = getAuthApi()
      const { result, user } = await performLocalLogin(credentials, api)
      if (result.success && result.session_id) {
        setSessionId(result.session_id)
        setActiveUser(user)
        setAuthMode('local')
        setAuthStatus('authenticated')
      } else {
        setAuthStatus('unauthenticated')
      }
      return result
    } finally {
      setIsAuthenticating(false)
    }
  }, [])

  const lock = useCallback(() => {
    setAuthStatus('locked')
  }, [])

  const unlockWithPin = useCallback(
    async (pin: string): Promise<LoginResult> => {
      if (!activeUser?.id) {
        setAuthStatus('unauthenticated')
        return { success: false }
      }
      setIsAuthenticating(true)
      try {
        const api = getAuthApi()
        const { result, user } = await performPinUnlock(
          activeUser.id,
          pin,
          api,
          activeUser.branch_id,
        )
        if (result.success && result.session_id) {
          setSessionId(result.session_id)
          if (user) {
            setActiveUser((prev) => (prev ? { ...prev, ...user } : user))
          }
          setAuthStatus('authenticated')
        }
        return result
      } finally {
        setIsAuthenticating(false)
      }
    },
    [activeUser?.id, activeUser?.branch_id],
  )

  const logout = useCallback(async (): Promise<void> => {
    const api = getAuthApi()
    await performLogout(sessionId, authMode, api)
    setSessionId(null)
    setActiveUser(null)
    setAuthStatus('unauthenticated')
  }, [sessionId, authMode])

  const value = useMemo<AuthContextType>(
    () => ({
      authStatus,
      authMode,
      setAuthMode,
      activeUser,
      sessionId,
      isAuthenticating,
      loginOnline,
      refreshSession,
      loginLocal,
      lock,
      unlockWithPin,
      logout,
    }),
    [
      authStatus,
      authMode,
      activeUser,
      sessionId,
      isAuthenticating,
      loginOnline,
      refreshSession,
      loginLocal,
      lock,
      unlockWithPin,
      logout,
    ],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth(): AuthContextType {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}
