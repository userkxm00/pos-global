// Terminal Lock Screen Component
// F1.14 — Local POS PIN Authentication and Lock Screen

import React, { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../../context/AuthContext'
import { useShell } from '../../context/ShellContext'
import { sanitizeAuthErrorMessage, validatePinInput } from '../auth/validation'
import { LockIcon } from '../common/Icons'
import { handleLockScreenKeyDown } from './lockHandlers'
import '../../styles/lock.css'

const PIN_SLOT_DEFINITIONS = Array.from({ length: 16 }, (_, i) => ({
  id: `pin-slot-pos-${i + 1}`,
  index: i,
}))

export const LockScreen: React.FC = () => {
  const { t } = useTranslation()
  const { activeUser, unlockWithPin, logout, isAuthenticating } = useAuth()
  const { organization, branch, register } = useShell()

  const [pin, setPin] = useState<string>('')
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false)

  const handleDigit = useCallback(
    (digit: string) => {
      setErrorMessage(null)
      setPin((prev) => (prev.length < 8 ? prev + digit : prev))
    },
    [],
  )

  const handleClear = useCallback(() => {
    setPin('')
    setErrorMessage(null)
  }, [])

  const handleBackspace = useCallback(() => {
    setPin((prev) => prev.slice(0, -1))
    setErrorMessage(null)
  }, [])

  const handleSubmit = useCallback(
    async (e?: React.FormEvent) => {
      if (e) e.preventDefault()
      if (isSubmitting || isAuthenticating) return

      const validationError = validatePinInput(pin)
      if (validationError) {
        setErrorMessage(t(validationError))
        return
      }

      setIsSubmitting(true)
      setErrorMessage(null)

      try {
        const result = await unlockWithPin(pin)
        if (!result.success) {
          setErrorMessage(t('auth.errors.invalidPin'))
        }
      } catch (err) {
        const errorKey = sanitizeAuthErrorMessage(err)
        setErrorMessage(t(errorKey))
      } finally {
        setPin('')
        setIsSubmitting(false)
      }
    },
    [pin, isSubmitting, isAuthenticating, unlockWithPin, t],
  )

  // Listen to physical keyboard events on window
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      handleLockScreenKeyDown(event, {
        onDigit: handleDigit,
        onBackspace: handleBackspace,
        onClear: handleClear,
        onSubmit: () => {
          void handleSubmit()
        },
      })
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [handleDigit, handleBackspace, handleClear, handleSubmit])

  const operatorName = activeUser?.full_name || activeUser?.username || activeUser?.email || t('lock.operator')
  const operatorRole = activeUser?.role || 'cashier'
  const orgName = organization?.name || 'POS Global'
  const branchName = branch?.name || 'Main Branch'
  const registerName = register?.name || 'Terminal 01'

  // Render 4 minimum visual dots, expanding as user types
  const visibleDots = PIN_SLOT_DEFINITIONS.slice(0, Math.max(4, pin.length))

  return (
    <main className="lock-wrapper" data-testid="terminal-lock-screen">
      <section className="lock-card" aria-labelledby="lock-title">
        <header className="lock-header">
          <div className="lock-icon-badge" aria-hidden="true">
            <LockIcon size={24} strokeWidth={2} />
          </div>
          <h1 id="lock-title" className="lock-title">
            {t('lock.title')}
          </h1>
          <p className="lock-subtitle">{t('lock.subtitle')}</p>

          {/* Context Badge */}
          <div className="lock-context-badge" aria-label={t('lock.tenantContext')}>
            <span>{orgName}</span>
            <span aria-hidden="true">/</span>
            <span>{branchName}</span>
            <span aria-hidden="true">•</span>
            <span>{registerName}</span>
          </div>

          {/* Operator Info */}
          <div className="lock-operator-info" aria-label={t('lock.activeOperator')}>
            <span className="lock-operator-name">{operatorName}</span>
            <span className="lock-operator-role">{operatorRole}</span>
          </div>
        </header>

        {/* Error Output */}
        {errorMessage && (
          <output role="alert" className="lock-error" aria-live="polite" data-testid="lock-error-message">
            {errorMessage}
          </output>
        )}

        {/* PIN Display Dots */}
        <div className="pin-display-wrapper" aria-label={t('lock.enterPin')}>
          <div className="pin-dots" aria-hidden="true">
            {visibleDots.map((slot) => (
              <div
                key={slot.id}
                className={`pin-dot ${slot.index < pin.length ? 'pin-dot--filled' : ''}`}
                data-testid={`pin-dot-${slot.index}`}
              />
            ))}
          </div>
        </div>

        {/* Numeric Keypad */}
        <form className="pin-form" onSubmit={handleSubmit} noValidate>
          <fieldset className="pin-keypad" aria-label={t('pin.pad')}>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('1')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-1"
            >
              1
            </button>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('2')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-2"
            >
              2
            </button>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('3')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-3"
            >
              3
            </button>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('4')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-4"
            >
              4
            </button>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('5')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-5"
            >
              5
            </button>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('6')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-6"
            >
              6
            </button>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('7')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-7"
            >
              7
            </button>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('8')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-8"
            >
              8
            </button>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('9')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-9"
            >
              9
            </button>
            <button
              type="button"
              className="pin-key pin-key--action"
              onClick={handleClear}
              disabled={isSubmitting || isAuthenticating || pin.length === 0}
              data-testid="pin-key-clear"
              aria-label={t('pin.clear')}
            >
              {t('pin.clear')}
            </button>
            <button
              type="button"
              className="pin-key"
              onClick={() => handleDigit('0')}
              disabled={isSubmitting || isAuthenticating}
              data-testid="pin-key-0"
            >
              0
            </button>
            <button
              type="button"
              className="pin-key pin-key--action"
              onClick={handleBackspace}
              disabled={isSubmitting || isAuthenticating || pin.length === 0}
              data-testid="pin-key-backspace"
              aria-label={t('pin.backspace')}
            >
              ⌫
            </button>
          </fieldset>

          {/* Action Buttons */}
          <div className="lock-actions">
            <button
              type="submit"
              className="btn btn--primary"
              disabled={isSubmitting || isAuthenticating || pin.length === 0}
              data-testid="pin-submit-button"
            >
              {isSubmitting || isAuthenticating ? t('lock.unlocking') : t('pin.unlock')}
            </button>

            <button
              type="button"
              className="btn btn--ghost btn--sm"
              onClick={logout}
              disabled={isSubmitting || isAuthenticating}
              data-testid="lock-switch-account-button"
            >
              {t('lock.switchAccount')}
            </button>
          </div>
        </form>
      </section>
    </main>
  )
}
