import React from 'react'
import { useTranslation } from 'react-i18next'
import type { Organization } from '../../types/organization'
import type { Branch } from '../../types/branch'
import type { Register } from '../../types/register'
import { PosIcon } from '../common/Icons'

interface CompletionStepProps {
  organization: Organization
  branch: Branch
  register: Register
  onLaunch: () => void
}

export const CompletionStep: React.FC<CompletionStepProps> = ({
  organization,
  branch,
  register,
  onLaunch,
}) => {
  const { t } = useTranslation()

  return (
    <div className="step-content">
      <div className="step-content__header">
        <h2 className="step-content__title">{t('onboarding.complete.title')}</h2>
        <p className="step-content__description">{t('onboarding.complete.description')}</p>
      </div>

      <div className="summary-card">
        <h3 style={{ margin: 0, fontSize: 'var(--font-size-md)', color: 'var(--color-text-primary)' }}>
          {t('onboarding.complete.summaryTitle')}
        </h3>

        <div className="summary-row">
          <span className="summary-row__label">{t('onboarding.complete.orgLabel')}</span>
          <span className="summary-row__value">{organization.name}</span>
        </div>

        <div className="summary-row">
          <span className="summary-row__label">{t('onboarding.complete.branchLabel')}</span>
          <span className="summary-row__value">{branch.name}</span>
        </div>

        <div className="summary-row">
          <span className="summary-row__label">{t('onboarding.complete.registerLabel')}</span>
          <span className="summary-row__value">
            {register.name} {register.code ? `(${register.code})` : ''}
          </span>
        </div>

        <div className="summary-row">
          <span className="summary-row__label">{t('onboarding.complete.currencyLabel')}</span>
          <span className="summary-row__value">{branch.currency || organization.default_currency}</span>
        </div>
      </div>

      <div className="onboarding-actions">
        <button
          type="button"
          className="btn btn--primary"
          onClick={onLaunch}
          style={{ width: '100%', justifyContent: 'center' }}
        >
          <PosIcon size={18} />
          <span>{t('onboarding.complete.launchAction')}</span>
        </button>
      </div>
    </div>
  )
}
