import React, { useCallback, useEffect, useState } from 'react'
import { ShellProvider, useShell } from './context/ShellContext'
import { AuthProvider, useAuth } from './context/AuthContext'
import { AppShell } from './components/shell/AppShell'
import { LoginScreen } from './components/auth/LoginScreen'
import { LockScreen } from './components/lock/LockScreen'
import { OnboardingWizard } from './components/onboarding/OnboardingWizard'
import { LoadingSkeleton } from './components/common/LoadingSkeleton'
import { getOnboardingApi } from './services/onboardingApi'
import { useInactivityTimeout } from './hooks/useInactivityTimeout'
import type { Organization } from './types/organization'
import type { Branch } from './types/branch'
import type { Register } from './types/register'
import './i18n'
import './styles/global.css'

export const AppContent: React.FC = () => {
  const {
    organization,
    branch,
    register,
    setOrganization,
    setBranch,
    setRegister,
    setActiveRoute,
  } = useShell()

  const { authStatus, activeUser, lock } = useAuth()
  const [isHydrating, setIsHydrating] = useState(false)

  // Inactivity timeout handler
  useInactivityTimeout({
    onTimeout: lock,
    isEnabled: authStatus === 'authenticated',
  })

  // Clear stale shell context whenever authentication is lost or revoked
  useEffect(() => {
    if (authStatus === 'unauthenticated' || authStatus === 'expired') {
      setOrganization(null)
      setBranch(null)
      setRegister(null)
    }
  }, [authStatus, setOrganization, setBranch, setRegister])

  // Hydrate persisted context only after a user is authenticated
  useEffect(() => {
    let isMounted = true

    async function hydrateContext() {
      if (authStatus !== 'authenticated') {
        return
      }

      setIsHydrating(true)
      try {
        const api = getOnboardingApi()
        const orgs = await api.listOrganizations()
        if (orgs.length > 0 && isMounted) {
          const primaryOrg = orgs[0]
          setOrganization(primaryOrg)

          const branches = await api.listBranches(primaryOrg.id)
          if (branches.length > 0 && isMounted) {
            const primaryBranch = branches[0]
            setBranch(primaryBranch)

            const registers = await api.listRegisters(primaryBranch.id)
            if (registers.length > 0 && isMounted) {
              setRegister(registers[0])
            }
          }
        }
      } catch {
        // Fail-closed; on error proceed with default empty state
      } finally {
        if (isMounted) {
          setIsHydrating(false)
        }
      }
    }

    void hydrateContext()

    return () => {
      isMounted = false
    }
  }, [authStatus, activeUser?.id, setOrganization, setBranch, setRegister])

  const handleOnboardingComplete = useCallback(
    (createdOrg: Organization, createdBranch: Branch, createdRegister: Register) => {
      setOrganization(createdOrg)
      setBranch(createdBranch)
      setRegister(createdRegister)
      setActiveRoute('pos')
    },
    [setOrganization, setBranch, setRegister, setActiveRoute],
  )

  if (authStatus === 'authenticating' || isHydrating) {
    return (
      <main className="onboarding-wrapper" data-testid="app-hydrating">
        <LoadingSkeleton cardsCount={3} />
      </main>
    )
  }

  // If terminal is locked, display LockScreen
  if (authStatus === 'locked') {
    return <LockScreen />
  }

  // If user is unauthenticated or session has expired, display the LoginScreen
  if (authStatus === 'unauthenticated' || authStatus === 'expired') {
    return <LoginScreen />
  }

  const isConfigured = Boolean(organization && branch && register)

  if (!isConfigured) {
    return (
      <OnboardingWizard
        initialOrganization={organization}
        initialBranch={branch}
        initialRegister={register}
        onComplete={handleOnboardingComplete}
      />
    )
  }

  return <AppShell />
}

export default function App() {
  return (
    <ShellProvider>
      <AuthProvider>
        <AppContent />
      </AuthProvider>
    </ShellProvider>
  )
}
