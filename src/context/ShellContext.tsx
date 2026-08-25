import React, { createContext, useContext, useEffect, useState, useMemo, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { getDirectionForLocale, SupportedLocale, defaultLocale } from '../i18n'
import type { Organization } from '../types/organization'
import type { Branch } from '../types/branch'
import type { Register } from '../types/register'
import type { SessionContext } from '../types/session'
import type { Permission } from '../types/permission'

export type NavigationRoute =
  | 'pos'
  | 'shifts'
  | 'inventory'
  | 'customers'
  | 'reports'
  | 'users'
  | 'tenants'
  | 'settings'

export type ViewState = 'idle' | 'loading' | 'empty' | 'error' | 'permission-denied'

export interface ShellContextType {
  activeRoute: NavigationRoute
  setActiveRoute: (route: NavigationRoute) => void
  sidebarCollapsed: boolean
  setSidebarCollapsed: (collapsed: boolean) => void
  toggleSidebar: () => void
  locale: SupportedLocale
  direction: 'ltr' | 'rtl'
  setLocale: (locale: SupportedLocale) => void
  organization: Organization | null
  setOrganization: (org: Organization | null) => void
  branch: Branch | null
  setBranch: (branch: Branch | null) => void
  register: Register | null
  setRegister: (register: Register | null) => void
  session: SessionContext | null
  setSession: (session: SessionContext | null) => void
  isOnline: boolean
  setIsOnline: (online: boolean) => void
  isSyncing: boolean
  setIsSyncing: (syncing: boolean) => void
  pendingSyncCount: number
  setPendingSyncCount: (count: number) => void
  viewState: ViewState
  setViewState: (state: ViewState) => void
  errorMessage: string | null
  setErrorMessage: (message: string | null) => void
  deniedPermission: Permission | null
  setDeniedPermission: (permission: Permission | null) => void
  lockSession: () => void
  logout: () => void
}

const ShellContext = createContext<ShellContextType | null>(null)

export interface ShellProviderProps {
  children: React.ReactNode
  initialRoute?: NavigationRoute
  initialOrganization?: Organization | null
  initialBranch?: Branch | null
  initialRegister?: Register | null
  initialSession?: SessionContext | null
  initialOnline?: boolean
}

export const ShellProvider: React.FC<ShellProviderProps> = ({
  children,
  initialRoute = 'pos',
  initialOrganization = null,
  initialBranch = null,
  initialRegister = null,
  initialSession = null,
  initialOnline = true,
}) => {
  const { i18n } = useTranslation()
  const [activeRoute, setActiveRoute] = useState<NavigationRoute>(initialRoute)
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false)
  const [locale, setLocale] = useState<SupportedLocale>((i18n.language as SupportedLocale) || defaultLocale)
  const [organization, setOrganization] = useState<Organization | null>(initialOrganization)
  const [branch, setBranch] = useState<Branch | null>(initialBranch)
  const [register, setRegister] = useState<Register | null>(initialRegister)
  const [session, setSession] = useState<SessionContext | null>(initialSession)
  const [isOnline, setIsOnline] = useState<boolean>(initialOnline)
  const [isSyncing, setIsSyncing] = useState<boolean>(false)
  const [pendingSyncCount, setPendingSyncCount] = useState<number>(0)
  const [viewState, setViewState] = useState<ViewState>('idle')
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [deniedPermission, setDeniedPermission] = useState<Permission | null>(null)

  const direction = useMemo(() => getDirectionForLocale(locale), [locale])

  const handleLocaleChange = useCallback(
    (newLocale: SupportedLocale) => {
      setLocale(newLocale)
      void i18n.changeLanguage(newLocale)
      document.documentElement.dir = getDirectionForLocale(newLocale)
      document.documentElement.lang = newLocale
    },
    [i18n],
  )

  const toggleSidebar = useCallback(() => {
    setSidebarCollapsed((prev) => !prev)
  }, [])

  const lockSession = useCallback(() => {
    setViewState('idle')
  }, [])

  const logout = useCallback(() => {
    setViewState('idle')
  }, [])

  // Listen to browser online/offline events
  useEffect(() => {
    const handleOnline = () => setIsOnline(true)
    const handleOffline = () => setIsOnline(false)

    window.addEventListener('online', handleOnline)
    window.addEventListener('offline', handleOffline)

    return () => {
      window.removeEventListener('online', handleOnline)
      window.removeEventListener('offline', handleOffline)
    }
  }, [])

  // Synchronize document direction attribute with locale
  useEffect(() => {
    document.documentElement.dir = direction
    document.documentElement.lang = locale
  }, [direction, locale])

  const value = useMemo<ShellContextType>(
    () => ({
      activeRoute,
      setActiveRoute,
      sidebarCollapsed,
      setSidebarCollapsed,
      toggleSidebar,
      locale,
      direction,
      setLocale: handleLocaleChange,
      organization,
      setOrganization,
      branch,
      setBranch,
      register,
      setRegister,
      session,
      setSession,
      isOnline,
      setIsOnline,
      isSyncing,
      setIsSyncing,
      pendingSyncCount,
      setPendingSyncCount,
      viewState,
      setViewState,
      errorMessage,
      setErrorMessage,
      deniedPermission,
      setDeniedPermission,
      lockSession,
      logout,
    }),
    [
      activeRoute,
      sidebarCollapsed,
      toggleSidebar,
      locale,
      direction,
      handleLocaleChange,
      organization,
      branch,
      register,
      session,
      isOnline,
      isSyncing,
      pendingSyncCount,
      viewState,
      errorMessage,
      deniedPermission,
      lockSession,
      logout,
    ],
  )

  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>
}

export function useShell(): ShellContextType {
  const context = useContext(ShellContext)
  if (!context) {
    throw new Error('useShell must be used within a ShellProvider')
  }
  return context
}
