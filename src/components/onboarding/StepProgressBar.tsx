import React from 'react'
import { useTranslation } from 'react-i18next'

export type OnboardingStepId = 'organization' | 'branch' | 'register' | 'complete'

interface StepProgressBarProps {
  currentStep: OnboardingStepId
}

interface StepMeta {
  id: OnboardingStepId
  number: number
  labelKey: string
}

const STEPS: StepMeta[] = [
  { id: 'organization', number: 1, labelKey: 'onboarding.steps.organization' },
  { id: 'branch', number: 2, labelKey: 'onboarding.steps.branch' },
  { id: 'register', number: 3, labelKey: 'onboarding.steps.register' },
  { id: 'complete', number: 4, labelKey: 'onboarding.steps.complete' },
]

export const StepProgressBar: React.FC<StepProgressBarProps> = ({ currentStep }) => {
  const { t } = useTranslation()
  const currentIndex = STEPS.findIndex((s) => s.id === currentStep)

  return (
    <nav className="step-progress-nav" aria-label={t('onboarding.title')}>
      <ol className="step-progress">
        {STEPS.map((step, index) => {
          const isActive = step.id === currentStep
          const isCompleted = index < currentIndex
          const itemClass = [
            'step-progress__item',
            isActive ? 'step-progress__item--active' : '',
            isCompleted ? 'step-progress__item--completed' : '',
          ]
            .filter(Boolean)
            .join(' ')

          return (
            <li
              key={step.id}
              className={itemClass}
              aria-current={isActive ? 'step' : undefined}
            >
              <div className="step-progress__circle" aria-hidden="true">
                {isCompleted ? '✓' : step.number}
              </div>
              <span className="step-progress__label">{t(step.labelKey)}</span>
            </li>
          )
        })}
      </ol>
    </nav>
  )
}
