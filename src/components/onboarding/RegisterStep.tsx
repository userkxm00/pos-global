import React, { useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import type { Organization } from '../../types/organization'
import type { Branch } from '../../types/branch'
import type { CreateRegisterInput, Register } from '../../types/register'
import { validateRegisterInput, ValidationErrors, sanitizeErrorMessage } from './validation'
import { getOnboardingApi } from '../../services/onboardingApi'

interface RegisterStepProps {
  organization: Organization
  branch: Branch
  onSuccess: (register: Register) => void
  onBack: () => void
  existingRegister?: Register | null
}

export const RegisterStep: React.FC<RegisterStepProps> = ({
  organization,
  branch,
  onSuccess,
  onBack,
  existingRegister,
}) => {
  const { t } = useTranslation()
  const [name, setName] = useState(existingRegister?.name || '')
  const [code, setCode] = useState(existingRegister?.code || '')
  const [errors, setErrors] = useState<ValidationErrors>({})
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [apiError, setApiError] = useState<string | null>(null)

  const handleNameChange = (val: string) => {
    setName(val)
    if (errors.name) setErrors((prev) => ({ ...prev, name: '' }))
  }

  const handleCodeChange = (val: string) => {
    setCode(val)
    if (errors.code) setErrors((prev) => ({ ...prev, code: '' }))
  }

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault()
      setApiError(null)

      const trimmedName = name.trim()
      const trimmedCode = code.trim() || null

      const input: CreateRegisterInput = {
        organization_id: organization.id,
        branch_id: branch.id,
        name: trimmedName,
        code: trimmedCode,
        is_active: true,
      }

      const validationErrors = validateRegisterInput(input)
      if (Object.keys(validationErrors).length > 0) {
        setErrors(validationErrors)
        return
      }
      setErrors({})

      // Idempotency check: if register was already created and data is unchanged, avoid re-creation
      if (
        existingRegister?.organization_id === organization.id &&
        existingRegister?.branch_id === branch.id &&
        existingRegister?.name === trimmedName &&
        (existingRegister?.code ?? null) === trimmedCode
      ) {
        onSuccess(existingRegister)
        return
      }

      setIsSubmitting(true)
      try {
        const api = getOnboardingApi()
        const createdReg = await api.createRegister(input)
        onSuccess(createdReg)
      } catch (err) {
        const safeErrorKey = sanitizeErrorMessage(err)
        setApiError(safeErrorKey)
      } finally {
        setIsSubmitting(false)
      }
    },
    [organization.id, branch.id, name, code, existingRegister, onSuccess],
  )

  return (
    <form className="step-content" onSubmit={handleSubmit} noValidate>
      <div className="step-content__header">
        <h2 className="step-content__title">{t('onboarding.register.title')}</h2>
        <p className="step-content__description">{t('onboarding.register.description')}</p>
      </div>

      {/* Context Badges */}
      <div className="context-pill-container">
        <div className="context-pill">
          <span>{t('onboarding.complete.orgLabel')}</span>
          <span className="context-pill__label">{organization.name}</span>
        </div>
        <div className="context-pill">
          <span>{t('onboarding.complete.branchLabel')}</span>
          <span className="context-pill__label">{branch.name}</span>
        </div>
      </div>

      {apiError && (
        <div className="alert-banner alert-banner--error" role="alert">
          <span>{t(apiError, { defaultValue: apiError })}</span>
        </div>
      )}

      {/* Register Name */}
      <div className="form-group">
        <label htmlFor="reg-name-input" className="form-label">
          {t('onboarding.register.nameLabel')} *
        </label>
        <input
          id="reg-name-input"
          type="text"
          className={`form-input ${errors.name ? 'form-input--error' : ''}`}
          placeholder={t('onboarding.register.namePlaceholder')}
          value={name}
          onChange={(e) => handleNameChange(e.target.value)}
          disabled={isSubmitting}
          aria-invalid={Boolean(errors.name)}
          aria-describedby={errors.name ? 'reg-name-error' : undefined}
          required
        />
        {errors.name && (
          <span id="reg-name-error" className="form-error">
            {t(errors.name)}
          </span>
        )}
      </div>

      {/* Device Code */}
      <div className="form-group">
        <label htmlFor="reg-code-input" className="form-label">
          {t('onboarding.register.codeLabel')}
        </label>
        <input
          id="reg-code-input"
          type="text"
          className={`form-input ${errors.code ? 'form-input--error' : ''}`}
          placeholder={t('onboarding.register.codePlaceholder')}
          value={code}
          onChange={(e) => handleCodeChange(e.target.value)}
          disabled={isSubmitting}
          aria-invalid={Boolean(errors.code)}
          aria-describedby={errors.code ? 'reg-code-error' : undefined}
        />
        <span className="form-hint">{t('onboarding.register.codeHint')}</span>
        {errors.code && <span className="form-error">{t(errors.code)}</span>}
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
          {isSubmitting
            ? t('onboarding.actions.submitting')
            : t('onboarding.actions.createRegister')}
        </button>
      </div>
    </form>
  )
}
