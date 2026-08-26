// AuthContext managing authentication lifecycle, session restoration, lock/unlock, and logout
// F1.13 — Authentication screens and session lifecycle & F1.14 — Local PIN and Lock screen

import React, { createContext, useContext, useEffect, useState, useMemo, useCallback } from 'react'
import type { AuthMode, AuthStatus, SignInInput, LocalSignInInput, OnlineSession } from '../types/auth'
import type { LoginResult } from '../types/session'
import { getAuthApi } from '../services/authApi'
import { DEFAULT_AUTH_MODE } from '../components/auth/constants'
import {
  AuthenticatedUser,
  evaluateStoredSession,
  performOnlineLogin,
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

  // Restore session on startup
  useEffect(() => {
    let isMounted = true

    async function initSession() {
      if (initialUser && initialSessionId) {
        if (isMounted) setAuthStatus('authenticated')
        return
      }

      const api = getAuthApi()
      const restored = await evaluateStoredSession(api)
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
