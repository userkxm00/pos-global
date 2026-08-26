// AuthContext managing authentication lifecycle, session restoration, lock/unlock, logout, and token refresh
// F1.13 — Authentication screens and session lifecycle & F1.14 — Local PIN and Lock screen & F1.19 — Supabase Auth adapter hardening
import React, { createContext, useContext, useEffect, useState, useMemo, useCallback, useRef } from 'react'
import type { AuthMode, AuthStatus, SignInInput, LocalSignInInput, OnlineSession } from '../types/auth'
import type { LoginResult } from '../types/session'
import { getAuthApi, classifyAuthError } from '../services/authApi'
import { DEFAULT_AUTH_MODE } from '../components/auth/constants'
import {
  AUTH_STORAGE_KEYS,
  AuthenticatedUser,
  clearStoredAuth,
  evaluateStoredSession,
  isTokenExpiringSoon,
  performOnlineLogin,
  performTokenRefresh,
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

  const isRefreshingRef = useRef<boolean>(false)

  const refreshSession = useCallback(async (): Promise<OnlineSession | null> => {
    if (isRefreshingRef.current) return null
    if (typeof window === 'undefined' || !window.sessionStorage) return null

    const storedMode = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.AUTH_MODE)
    const storedRefreshToken = window.sessionStorage.getItem(AUTH_STORAGE_KEYS.CLOUD_REFRESH_TOKEN)

    if (storedMode !== 'online' || !storedRefreshToken) {
      return null
    }

    isRefreshingRef.current = true
    try {
      const api = getAuthApi()
      const { session, user } = await performTokenRefresh(storedRefreshToken, api, activeUser)
      setActiveUser(user)
      setSessionId(session.user.id)
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
      isRefreshingRef.current = false
    }
  }, [activeUser])

  // Restore session on startup with immediate expiry evaluation
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

      if (restored.mode === 'online' && restored.user && restored.sessionId) {
        // If restored token is already expired or expiring soon, evaluate immediately
        if (restored.status === 'expired' || isTokenExpiringSoon(restored.expiresAt)) {
          if (restored.refreshToken) {
            try {
              const { session, user } = await performTokenRefresh(restored.refreshToken, api, restored.user)
              if (isMounted) {
                setActiveUser(user)
                setSessionId(session.user.id)
                setAuthMode('online')
                setAuthStatus('authenticated')
              }
              return
            } catch (err: unknown) {
              const typedErr = classifyAuthError(err)
              if (isMounted) {
                if (typedErr.code === 'network_error') {
                  // On network failure during startup refresh, fail closed for cloud session while preserving local capability
                  setAuthStatus('unauthenticated')
                  setActiveUser(null)
                  setSessionId(null)
                } else {
                  clearStoredAuth()
                  setAuthStatus('expired')
                  setActiveUser(null)
                  setSessionId(null)
                }
              }
              return
            }
          } else {
            // Expired with no refresh token: fail closed
            clearStoredAuth()
            if (isMounted) {
              setAuthStatus('expired')
              setActiveUser(null)
              setSessionId(null)
            }
            return
          }
        }
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
  }, [initialUser, initialSessionId])

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
