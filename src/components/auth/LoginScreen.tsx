// Accessible, tokenized LoginScreen supporting Online Supabase & Local POS Sign-In
// F1.13 — Authentication screens and session lifecycle

import React, { useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../../context/AuthContext'
import { useShell } from '../../context/ShellContext'
import { validateOnlineInput, validateLocalInput, sanitizeAuthErrorMessage, AuthValidationErrors } from './validation'
import '../../styles/auth.css'

export const LoginScreen: React.FC = () => {
  const { t } = useTranslation()
  const { authStatus, authMode, setAuthMode, loginOnline, loginLocal, isAuthenticating } = useAuth()
  const { isOnline } = useShell()

  const [email, setEmail] = useState('')
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [errors, setErrors] = useState<AuthValidationErrors>({})
  const [apiError, setApiError] = useState<string | null>(
    authStatus === 'expired' ? 'auth.sessionExpiredNotice' : null,
  )

  const handleModeChange = (mode: 'online' | 'local') => {
    setAuthMode(mode)
    setErrors({})
    setApiError(null)
    setPassword('')
  }

  const handleEmailChange = (val: string) => {
    setEmail(val)
    if (errors.email) setErrors((prev) => ({ ...prev, email: undefined }))
  }

  const handleUsernameChange = (val: string) => {
    setUsername(val)
    if (errors.username) setErrors((prev) => ({ ...prev, username: undefined }))
  }

  const handlePasswordChange = (val: string) => {
    setPassword(val)
    if (errors.password) setErrors((prev) => ({ ...prev, password: undefined }))
  }

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault()
      setApiError(null)

      if (authMode === 'online') {
        const validation = validateOnlineInput({ email, password })
        if (Object.keys(validation).length > 0) {
          setErrors(validation)
          return
        }
        setErrors({})

        try {
          await loginOnline({ email: email.trim(), password })
        } catch (err) {
          const safeErrorKey = sanitizeAuthErrorMessage(err)
          setApiError(safeErrorKey)
        }
      } else {
        const validation = validateLocalInput({ username, password })
        if (Object.keys(validation).length > 0) {
          setErrors(validation)
          return
        }
        setErrors({})

        try {
          const result = await loginLocal({ username: username.trim(), password })
          if (!result.success) {
            setApiError('auth.errors.invalidCredentials')
          }
        } catch (err) {
          const safeErrorKey = sanitizeAuthErrorMessage(err)
          setApiError(safeErrorKey)
        }
      }
    },
    [authMode, email, username, password, loginOnline, loginLocal],
  )

  return (
    <main className="auth-wrapper">
      <section className="auth-card" aria-label={t('auth.title')}>
        <div className="auth-header">
          <h1 className="auth-brand">{t('app.name')}</h1>
          <h2 className="auth-title">{t('auth.title')}</h2>
          <p className="auth-subtitle">{t('auth.subtitle')}</p>
        </div>

        {/* Auth Mode Tabs */}
        <div className="auth-tabs" role="tablist" aria-label={t('auth.modes.title')}>
          <button
            type="button"
            role="tab"
            aria-selected={authMode === 'online'}
            className={`auth-tab ${authMode === 'online' ? 'auth-tab--active' : ''}`}
            onClick={() => handleModeChange('online')}
            disabled={isAuthenticating}
          >
            {t('auth.onlineTab')}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={authMode === 'local'}
            className={`auth-tab ${authMode === 'local' ? 'auth-tab--active' : ''}`}
            onClick={() => handleModeChange('local')}
            disabled={isAuthenticating}
          >
            {t('auth.localTab')}
          </button>
        </div>

        {/* Offline Notice */}
        {!isOnline && authMode === 'online' && (
          <output className="auth-notice">
            {t('auth.offlineNotice')}
          </output>
        )}

        {/* Error Alert */}
        {apiError && (
          <div className="alert-banner alert-banner--error" role="alert">
            <span>{t(apiError, { defaultValue: apiError })}</span>
          </div>
        )}

        {/* Login Form */}
        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {authMode === 'online' ? (
            <div className="form-group">
              <label htmlFor="auth-email-input" className="form-label">
                {t('auth.emailLabel')} *
              </label>
              <input
                id="auth-email-input"
                type="email"
                className={`form-input ${errors.email ? 'form-input--error' : ''}`}
                placeholder={t('auth.emailPlaceholder')}
                value={email}
                onChange={(e) => handleEmailChange(e.target.value)}
                disabled={isAuthenticating}
                aria-invalid={Boolean(errors.email)}
                aria-describedby={errors.email ? 'auth-email-error' : undefined}
                autoComplete="email"
                autoFocus
                required
              />
              {errors.email && (
                <span id="auth-email-error" className="form-error">
                  {t(errors.email)}
                </span>
              )}
            </div>
          ) : (
            <div className="form-group">
              <label htmlFor="auth-username-input" className="form-label">
                {t('auth.usernameLabel')} *
              </label>
              <input
                id="auth-username-input"
                type="text"
                className={`form-input ${errors.username ? 'form-input--error' : ''}`}
                placeholder={t('auth.usernamePlaceholder')}
                value={username}
                onChange={(e) => handleUsernameChange(e.target.value)}
                disabled={isAuthenticating}
                aria-invalid={Boolean(errors.username)}
                aria-describedby={errors.username ? 'auth-username-error' : undefined}
                autoComplete="username"
                autoFocus
                required
              />
              {errors.username && (
                <span id="auth-username-error" className="form-error">
                  {t(errors.username)}
                </span>
              )}
            </div>
          )}

          {/* Password Field */}
          <div className="form-group">
            <label htmlFor="auth-password-input" className="form-label">
              {t('auth.passwordLabel')} *
            </label>
            <input
              id="auth-password-input"
              type="password"
              className={`form-input ${errors.password ? 'form-input--error' : ''}`}
              placeholder={t('auth.passwordPlaceholder')}
              value={password}
              onChange={(e) => handlePasswordChange(e.target.value)}
              disabled={isAuthenticating}
              aria-invalid={Boolean(errors.password)}
              aria-describedby={errors.password ? 'auth-password-error' : undefined}
              autoComplete="current-password"
              required
            />
            {errors.password && (
              <span id="auth-password-error" className="form-error">
                {t(errors.password)}
              </span>
            )}
          </div>

          {/* Submit Action */}
          <button
            type="submit"
            className="btn btn--primary btn--full"
            disabled={isAuthenticating}
          >
            {isAuthenticating ? t('auth.signingIn') : t('auth.signInButton')}
          </button>
        </form>
      </section>
    </main>
  )
}
