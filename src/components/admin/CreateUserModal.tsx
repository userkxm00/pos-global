// Create User Modal Component
// F1.16 — Roles / Permissions Administration UI

import React, { useState, useEffect, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { getPermissionApi } from '../../services/permissionApi'
import type { Role } from '../../types/permission'
import type { User, CreateUserInput } from '../../types/user'
import { AUTHORITATIVE_ROLES } from '../../context/permissionEvaluation'

export interface CreateUserModalProps {
  isOpen: boolean
  branchId: string
  onClose: () => void
  onUserCreated: (user: User) => void
}

export const CreateUserModal: React.FC<CreateUserModalProps> = ({
  isOpen,
  branchId,
  onClose,
  onUserCreated,
}) => {
  const { t } = useTranslation()

  const [fullName, setFullName] = useState('')
  const [username, setUsername] = useState('')
  const [role, setRole] = useState<Role>('cashier')
  const [pin, setPin] = useState('')
  const [password, setPassword] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

  const dialogElementRef = useRef<HTMLDialogElement>(null)

  // Reset form fields
  const resetForm = useCallback(() => {
    setFullName('')
    setUsername('')
    setRole('cashier')
    setPin('')
    setPassword('')
    setErrorMessage(null)
    setIsSubmitting(false)
  }, [])

  // Native modal dialog lifecycle, non-dismissible submission, and focus restoration
  useEffect(() => {
    if (!isOpen) return

    const dialogNode = dialogElementRef.current
    if (!dialogNode) return

    const previousElement = document.activeElement as HTMLElement | null
    if (!dialogNode.open && typeof dialogNode.showModal === 'function') {
      dialogNode.showModal()
    }

    const onBackdropClick = (evt: MouseEvent) => {
      if (evt.target === dialogNode && !isSubmitting) {
        onClose()
      }
    }

    const onCancelModal = (evt: Event) => {
      evt.preventDefault()
      if (!isSubmitting) {
        onClose()
      }
    }

    dialogNode.addEventListener('click', onBackdropClick)
    dialogNode.addEventListener('cancel', onCancelModal)

    return () => {
      dialogNode.removeEventListener('click', onBackdropClick)
      dialogNode.removeEventListener('cancel', onCancelModal)
      if (dialogNode.open && typeof dialogNode.close === 'function') {
        dialogNode.close()
      }
      if (previousElement) {
        previousElement.focus()
      }
    }
  }, [isOpen, isSubmitting, onClose])

  // Reset form when opened
  useEffect(() => {
    if (isOpen) {
      resetForm()
    }
  }, [isOpen, resetForm])

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault()
      if (isSubmitting) return

      const trimmedName = fullName.trim()
      if (!trimmedName) {
        setErrorMessage(t('admin.users.errors.nameRequired'))
        return
      }
      if (trimmedName.length > 255) {
        setErrorMessage(t('admin.users.errors.nameTooLong'))
        return
      }

      if (pin && (pin.length < 4 || pin.length > 8 || !/^\d+$/.test(pin))) {
        setErrorMessage(t('admin.users.errors.invalidPin'))
        return
      }

      setIsSubmitting(true)
      setErrorMessage(null)

      try {
        const input: CreateUserInput = {
          branch_id: branchId,
          full_name: trimmedName,
          username: username.trim() || null,
          role,
          pin: pin.trim() || null,
          password: password || null,
        }

        const api = getPermissionApi()
        const created = await api.createUser(input)
        onUserCreated(created)
        onClose()
      } catch (err) {
        setErrorMessage(
          err instanceof Error && err.message ? err.message : t('admin.users.errors.createFailed'),
        )
      } finally {
        setIsSubmitting(false)
      }
    },
    [branchId, fullName, username, role, pin, password, isSubmitting, onUserCreated, onClose, t],
  )

  if (!isOpen) return null

  return (
    <div className="context-modal-backdrop" data-testid="create-user-backdrop">
      <dialog
        ref={dialogElementRef}
        className="context-modal"
        aria-modal="true"
        aria-labelledby="create-user-title"
        data-testid="create-user-modal"
      >
        <header className="context-modal__header">
          <div className="context-modal__titles">
            <h2 id="create-user-title" className="context-modal__title">
              {t('admin.users.createTitle')}
            </h2>
            <p className="context-modal__subtitle">{t('admin.users.createSubtitle')}</p>
          </div>
          <button
            type="button"
            className="context-modal__close-btn"
            onClick={onClose}
            disabled={isSubmitting}
            aria-label={t('admin.users.closeModal')}
            data-testid="create-user-close-btn"
          >
            ✕
          </button>
        </header>

        {errorMessage && (
          <div role="alert" className="context-modal__error" data-testid="create-user-error">
            <span>{errorMessage}</span>
          </div>
        )}

        <form className="context-modal__form" onSubmit={handleSubmit} noValidate>
          {/* Full Name */}
          <div className="context-modal__group">
            <label htmlFor="create-user-fullname" className="context-modal__label">
              {t('admin.users.fields.fullName')} *
            </label>
            <input
              id="create-user-fullname"
              type="text"
              className="form-input"
              value={fullName}
              onChange={(e) => setFullName(e.target.value)}
              placeholder={t('admin.users.placeholders.fullName')}
              disabled={isSubmitting}
              required
              data-testid="create-user-fullname-input"
            />
          </div>

          {/* Username */}
          <div className="context-modal__group">
            <label htmlFor="create-user-username" className="context-modal__label">
              {t('admin.users.fields.username')}
            </label>
            <input
              id="create-user-username"
              type="text"
              className="form-input"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder={t('admin.users.placeholders.username')}
              disabled={isSubmitting}
              data-testid="create-user-username-input"
            />
          </div>

          {/* Role Selection */}
          <div className="context-modal__group">
            <label htmlFor="create-user-role" className="context-modal__label">
              {t('admin.users.fields.role')} *
            </label>
            <select
              id="create-user-role"
              className="context-modal__select"
              value={role}
              onChange={(e) => setRole(e.target.value as Role)}
              disabled={isSubmitting}
              data-testid="create-user-role-select"
            >
              {AUTHORITATIVE_ROLES.map((r) => (
                <option key={r} value={r}>
                  {t(`roles.${r}.title`)}
                </option>
              ))}
            </select>
          </div>

          {/* POS PIN (Optional) */}
          <div className="context-modal__group">
            <label htmlFor="create-user-pin" className="context-modal__label">
              {t('admin.users.fields.pin')}
            </label>
            <input
              id="create-user-pin"
              type="password"
              className="form-input"
              value={pin}
              onChange={(e) => setPin(e.target.value)}
              placeholder={t('admin.users.placeholders.pin')}
              maxLength={8}
              disabled={isSubmitting}
              data-testid="create-user-pin-input"
            />
          </div>

          {/* Password (Optional) */}
          <div className="context-modal__group">
            <label htmlFor="create-user-password" className="context-modal__label">
              {t('admin.users.fields.password')}
            </label>
            <input
              id="create-user-password"
              type="password"
              className="form-input"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t('admin.users.placeholders.password')}
              disabled={isSubmitting}
              data-testid="create-user-password-input"
            />
          </div>

          {/* Actions */}
          <div className="context-modal__actions">
            <button
              type="button"
              className="btn btn--secondary"
              onClick={onClose}
              disabled={isSubmitting}
              data-testid="create-user-cancel-btn"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              className="btn btn--primary"
              disabled={isSubmitting || !fullName.trim()}
              data-testid="create-user-submit-btn"
            >
              {isSubmitting ? t('common.saving') : t('admin.users.createUserBtn')}
            </button>
          </div>
        </form>
      </dialog>
    </div>
  )
}
