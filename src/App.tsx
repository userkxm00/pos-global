import React, { useCallback, useEffect, useState } from 'react'
import { ShellProvider, useShell } from './context/ShellContext'
import { AppShell } from './components/shell/AppShell'
import { OnboardingWizard } from './components/onboarding/OnboardingWizard'
import { LoadingSkeleton } from './components/common/LoadingSkeleton'
import { getOnboardingApi } from './services/onboardingApi'
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

  const [isHydrating, setIsHydrating] = useState(true)

  // Hydrate persisted context on initial mount before evaluating onboarding gate
  useEffect(() => {
    let isMounted = true

    async function hydrateContext() {
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
  }, [setOrganization, setBranch, setRegister])

  const handleOnboardingComplete = useCallback(
    (createdOrg: Organization, createdBranch: Branch, createdRegister: Register) => {
      setOrganization(createdOrg)
      setBranch(createdBranch)
      setRegister(createdRegister)
      setActiveRoute('pos')
    },
    [setOrganization, setBranch, setRegister, setActiveRoute],
  )

  if (isHydrating) {
    return (
      <main className="onboarding-wrapper" data-testid="app-hydrating">
        <LoadingSkeleton cardsCount={3} />
      </main>
    )
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
      <AppContent />
    </ShellProvider>
  )
}
