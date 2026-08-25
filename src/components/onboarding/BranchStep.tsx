import React, { useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import type { Organization } from '../../types/organization'
import type { CreateBranchInput, Branch } from '../../types/branch'
import {
  validateBranchInput,
  ValidationErrors,
  sanitizeErrorMessage,
  normalizeCurrency,
} from './validation'
import { getOnboardingApi } from '../../services/onboardingApi'
import { getCurrencyOptions } from './constants'

interface BranchStepProps {
  organization: Organization
  onSuccess: (branch: Branch) => void
  onBack: () => void
  existingBranch?: Branch | null
}

export const BranchStep: React.FC<BranchStepProps> = ({
  organization,
  onSuccess,
  onBack,
  existingBranch,
}) => {
  const { t } = useTranslation()
  const [name, setName] = useState(existingBranch?.name || '')
  const [address, setAddress] = useState(existingBranch?.address || '')
  const [currency, setCurrency] = useState(
    existingBranch?.currency || organization.default_currency || 'USD',
  )
  const [errors, setErrors] = useState<ValidationErrors>({})
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [apiError, setApiError] = useState<string | null>(null)

  const currencyOptions = getCurrencyOptions(organization.default_currency)

  const handleNameChange = (val: string) => {
    setName(val)
    if (errors.name) setErrors((prev) => ({ ...prev, name: '' }))
  }

  const handleAddressChange = (val: string) => {
    setAddress(val)
    if (errors.address) setErrors((prev) => ({ ...prev, address: '' }))
  }

  const handleCurrencyChange = (val: string) => {
    setCurrency(val)
    if (errors.currency) setErrors((prev) => ({ ...prev, currency: '' }))
  }

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault()
      setApiError(null)

      const trimmedName = name.trim()
      const trimmedAddress = address.trim() || null
      const normalizedCurrency = normalizeCurrency(currency)

      const input: CreateBranchInput = {
        organization_id: organization.id,
        name: trimmedName,
        address: trimmedAddress,
        currency: normalizedCurrency,
        is_active: true,
      }

      const validationErrors = validateBranchInput(input)
      if (Object.keys(validationErrors).length > 0) {
        setErrors(validationErrors)
        return
      }
      setErrors({})

      // Idempotency check: if branch was already created and data is unchanged, avoid re-creation
      if (
        existingBranch?.organization_id === organization.id &&
        existingBranch?.name === trimmedName &&
        (existingBranch?.address ?? null) === trimmedAddress &&
        existingBranch?.currency === normalizedCurrency
      ) {
        onSuccess(existingBranch)
        return
      }

      setIsSubmitting(true)
      try {
        const api = getOnboardingApi()
        const createdBranch = await api.createBranch(input)
        onSuccess(createdBranch)
      } catch (err) {
        const safeErrorKey = sanitizeErrorMessage(err)
        setApiError(safeErrorKey)
      } finally {
        setIsSubmitting(false)
      }
    },
    [organization.id, name, address, currency, existingBranch, onSuccess],
  )

  return (
    <form className="step-content" onSubmit={handleSubmit} noValidate>
      <div className="step-content__header">
        <h2 className="step-content__title">{t('onboarding.branch.title')}</h2>
        <p className="step-content__description">{t('onboarding.branch.description')}</p>
      </div>

      {/* Organization Context Badge */}
      <div className="context-pill-container">
        <div className="context-pill">
          <span>{t('onboarding.complete.orgLabel')}</span>
          <span className="context-pill__label">{organization.name}</span>
        </div>
      </div>

      {apiError && (
        <div className="alert-banner alert-banner--error" role="alert">
          <span>{t(apiError, { defaultValue: apiError })}</span>
        </div>
      )}

      {/* Branch Name */}
      <div className="form-group">
        <label htmlFor="branch-name-input" className="form-label">
          {t('onboarding.branch.nameLabel')} *
        </label>
        <input
          id="branch-name-input"
          type="text"
          className={`form-input ${errors.name ? 'form-input--error' : ''}`}
          placeholder={t('onboarding.branch.namePlaceholder')}
          value={name}
          onChange={(e) => handleNameChange(e.target.value)}
          disabled={isSubmitting}
          aria-invalid={Boolean(errors.name)}
          aria-describedby={errors.name ? 'branch-name-error' : undefined}
          required
        />
        {errors.name && (
          <span id="branch-name-error" className="form-error">
            {t(errors.name)}
          </span>
        )}
      </div>

      {/* Address */}
      <div className="form-group">
        <label htmlFor="branch-address-input" className="form-label">
          {t('onboarding.branch.addressLabel')}
        </label>
        <input
          id="branch-address-input"
          type="text"
          className={`form-input ${errors.address ? 'form-input--error' : ''}`}
          placeholder={t('onboarding.branch.addressPlaceholder')}
          value={address}
          onChange={(e) => handleAddressChange(e.target.value)}
          disabled={isSubmitting}
          aria-invalid={Boolean(errors.address)}
          aria-describedby={errors.address ? 'branch-address-error' : undefined}
        />
        {errors.address && (
          <span id="branch-address-error" className="form-error">
            {t(errors.address)}
          </span>
        )}
      </div>

      {/* Branch Currency */}
      <div className="form-group">
        <label htmlFor="branch-currency-select" className="form-label">
          {t('onboarding.branch.currencyLabel')}
        </label>
        <select
          id="branch-currency-select"
          className={`form-select ${errors.currency ? 'form-select--error' : ''}`}
          value={currency}
          onChange={(e) => handleCurrencyChange(e.target.value)}
          disabled={isSubmitting}
        >
          {currencyOptions.map((curr) => (
            <option key={curr} value={curr}>
              {curr}
            </option>
          ))}
        </select>
        <span className="form-hint">{t('onboarding.branch.currencyHint')}</span>
        {errors.currency && <span className="form-error">{t(errors.currency)}</span>}
      </div>

      {/* Actions */}
      <div className="onboarding-actions onboarding-actions--space-between">
        <button
          type="button"
          className="btn btn--secondary"
          onClick={onBack}
          disabled={isSubmitting}
        >
          {t('onboarding.actions.back')}
        </button>
        <button
          type="submit"
          className="btn btn--primary"
          disabled={isSubmitting}
        >
          {isSubmitting ? t('onboarding.actions.submitting') : t('onboarding.actions.createBranch')}
        </button>
      </div>
    </form>
  )
}
