import React, { useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import type { CreateOrganizationInput, Organization } from '../../types/organization'
import {
  validateOrganizationInput,
  ValidationErrors,
  sanitizeErrorMessage,
  normalizeCurrency,
} from './validation'
import { getOnboardingApi } from '../../services/onboardingApi'
import { getLanguageOptions, getCurrencyOptions } from './constants'

interface OrganizationStepProps {
  onSuccess: (org: Organization) => void
  existingOrganization?: Organization | null
}

export const OrganizationStep: React.FC<OrganizationStepProps> = ({
  onSuccess,
  existingOrganization,
}) => {
  const { t } = useTranslation()
  const [name, setName] = useState(existingOrganization?.name || '')
  const [currency, setCurrency] = useState(existingOrganization?.default_currency || 'USD')
  const [language, setLanguage] = useState(existingOrganization?.default_language || 'en')
  const [errors, setErrors] = useState<ValidationErrors>({})
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [apiError, setApiError] = useState<string | null>(null)

  const currencyOptions = getCurrencyOptions(currency)
  const languageOptions = getLanguageOptions(existingOrganization?.default_language || language)

  const handleNameChange = (val: string) => {
    setName(val)
    if (errors.name) setErrors((prev) => ({ ...prev, name: '' }))
  }

  const handleCurrencyChange = (val: string) => {
    setCurrency(val)
    if (errors.default_currency) setErrors((prev) => ({ ...prev, default_currency: '' }))
  }

  const handleLanguageChange = (val: string) => {
    setLanguage(val)
    if (errors.default_language) setErrors((prev) => ({ ...prev, default_language: '' }))
  }

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault()
      setApiError(null)

      const trimmedName = name.trim()
      const normalizedCurrency = normalizeCurrency(currency)
      const trimmedLanguage = language.trim()

      const input: CreateOrganizationInput = {
        name: trimmedName,
        default_currency: normalizedCurrency,
        default_language: trimmedLanguage,
      }

      const validationErrors = validateOrganizationInput(input)
      if (Object.keys(validationErrors).length > 0) {
        setErrors(validationErrors)
        return
      }
      setErrors({})

      // Idempotency check: if organization was already created and data is unchanged, avoid re-creation
      if (
        existingOrganization?.name === trimmedName &&
        existingOrganization?.default_currency === normalizedCurrency &&
        existingOrganization?.default_language === trimmedLanguage
      ) {
        onSuccess(existingOrganization)
        return
      }

      setIsSubmitting(true)
      try {
        const api = getOnboardingApi()
        const createdOrg = await api.createOrganization(input)
        onSuccess(createdOrg)
      } catch (err) {
        const safeErrorKey = sanitizeErrorMessage(err)
        setApiError(safeErrorKey)
      } finally {
        setIsSubmitting(false)
      }
    },
    [name, currency, language, existingOrganization, onSuccess],
  )

  return (
    <form className="step-content" onSubmit={handleSubmit} noValidate>
      <div className="step-content__header">
        <h2 className="step-content__title">{t('onboarding.org.title')}</h2>
        <p className="step-content__description">{t('onboarding.org.description')}</p>
      </div>

      {apiError && (
        <div className="alert-banner alert-banner--error" role="alert">
          <span>{t(apiError, { defaultValue: apiError })}</span>
        </div>
      )}

      {/* Organization Name */}
      <div className="form-group">
        <label htmlFor="org-name-input" className="form-label">
          {t('onboarding.org.nameLabel')} *
        </label>
        <input
          id="org-name-input"
          type="text"
          className={`form-input ${errors.name ? 'form-input--error' : ''}`}
          placeholder={t('onboarding.org.namePlaceholder')}
          value={name}
          onChange={(e) => handleNameChange(e.target.value)}
          disabled={isSubmitting}
          aria-invalid={Boolean(errors.name)}
          aria-describedby={errors.name ? 'org-name-error' : undefined}
          required
        />
        {errors.name && (
          <span id="org-name-error" className="form-error">
            {t(errors.name)}
          </span>
        )}
      </div>

      {/* Default Currency */}
      <div className="form-group">
        <label htmlFor="org-currency-select" className="form-label">
          {t('onboarding.org.currencyLabel')}
        </label>
        <select
          id="org-currency-select"
          className={`form-select ${errors.default_currency ? 'form-select--error' : ''}`}
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
        <span className="form-hint">{t('onboarding.org.currencyHint')}</span>
        {errors.default_currency && (
          <span className="form-error">{t(errors.default_currency)}</span>
        )}
      </div>

      {/* Default Language */}
      <div className="form-group">
        <label htmlFor="org-language-select" className="form-label">
          {t('onboarding.org.languageLabel')}
        </label>
        <select
          id="org-language-select"
          className={`form-select ${errors.default_language ? 'form-select--error' : ''}`}
          value={language}
          onChange={(e) => handleLanguageChange(e.target.value)}
          disabled={isSubmitting}
        >
          {languageOptions.map((lang) => (
            <option key={lang.code} value={lang.code}>
              {lang.labelKey
                ? `${t(lang.labelKey)} (${lang.code.toUpperCase()})`
                : lang.fallbackLabel || lang.code.toUpperCase()}
            </option>
          ))}
        </select>
        <span className="form-hint">{t('onboarding.org.languageHint')}</span>
        {errors.default_language && (
          <span className="form-error">{t(errors.default_language)}</span>
        )}
      </div>

      {/* Actions */}
      <div className="onboarding-actions">
        <button
          type="submit"
          className="btn btn--primary"
          disabled={isSubmitting}
        >
          {isSubmitting ? t('onboarding.actions.submitting') : t('onboarding.actions.createOrg')}
        </button>
      </div>
    </form>
  )
}
