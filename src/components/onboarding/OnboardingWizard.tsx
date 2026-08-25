import React, { useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import type { Organization } from '../../types/organization'
import type { Branch } from '../../types/branch'
import type { Register } from '../../types/register'
import { StepProgressBar, OnboardingStepId } from './StepProgressBar'
import { OrganizationStep } from './OrganizationStep'
import { BranchStep } from './BranchStep'
import { RegisterStep } from './RegisterStep'
import { CompletionStep } from './CompletionStep'
import { PosIcon } from '../common/Icons'
import { determineInitialStep } from './constants'
import '../../styles/onboarding.css'

export { determineInitialStep } from './constants'

export interface OnboardingWizardProps {
  initialOrganization?: Organization | null
  initialBranch?: Branch | null
  initialRegister?: Register | null
  onComplete: (org: Organization, branch: Branch, register: Register) => void
}

export const OnboardingWizard: React.FC<OnboardingWizardProps> = ({
  initialOrganization = null,
  initialBranch = null,
  initialRegister = null,
  onComplete,
}) => {
  const { t } = useTranslation()
  const [organization, setOrganization] = useState<Organization | null>(initialOrganization)
  const [branch, setBranch] = useState<Branch | null>(initialBranch)
  const [register, setRegister] = useState<Register | null>(initialRegister)
  const [step, setStep] = useState<OnboardingStepId>(() =>
    determineInitialStep(initialOrganization, initialBranch, initialRegister),
  )

  const handleOrgSuccess = useCallback((createdOrg: Organization) => {
    setOrganization((prevOrg) => {
      if (prevOrg && prevOrg.id !== createdOrg.id) {
        setBranch(null)
        setRegister(null)
      }
      return createdOrg
    })
    setStep('branch')
  }, [])

  const handleBranchSuccess = useCallback((createdBranch: Branch) => {
    setBranch((prevBranch) => {
      if (prevBranch && prevBranch.id !== createdBranch.id) {
        setRegister(null)
      }
      return createdBranch
    })
    setStep('register')
  }, [])

  const handleRegisterSuccess = useCallback((createdRegister: Register) => {
    setRegister(createdRegister)
    setStep('complete')
  }, [])

  const handleLaunch = useCallback(() => {
    if (organization && branch && register) {
      onComplete(organization, branch, register)
    }
  }, [organization, branch, register, onComplete])

  return (
    <main className="onboarding-wrapper" data-testid="onboarding-wizard">
      <header className="onboarding-header">
        <div className="onboarding-header__logo" aria-hidden="true">
          <PosIcon size={32} strokeWidth={2.5} />
        </div>
        <h1 className="onboarding-header__title">{t('app.name')}</h1>
        <p className="onboarding-header__subtitle">{t('onboarding.subtitle')}</p>
      </header>

      <section className="onboarding-card" aria-labelledby="onboarding-flow-title">
        <h2 id="onboarding-flow-title" className="sr-only">
          {t('onboarding.title')}
        </h2>

        {/* Accessible Step Progress */}
        <StepProgressBar currentStep={step} />

        {/* Step Views */}
        {step === 'organization' && (
          <OrganizationStep
            existingOrganization={organization}
            onSuccess={handleOrgSuccess}
          />
        )}

        {step === 'branch' && organization && (
          <BranchStep
            organization={organization}
            existingBranch={branch}
            onSuccess={handleBranchSuccess}
            onBack={() => setStep('organization')}
          />
        )}

        {step === 'register' && organization && branch && (
          <RegisterStep
            organization={organization}
            branch={branch}
            existingRegister={register}
            onSuccess={handleRegisterSuccess}
            onBack={() => setStep('branch')}
          />
        )}

        {step === 'complete' && organization && branch && register && (
          <CompletionStep
            organization={organization}
            branch={branch}
            register={register}
            onLaunch={handleLaunch}
          />
        )}
      </section>
    </main>
  )
}
