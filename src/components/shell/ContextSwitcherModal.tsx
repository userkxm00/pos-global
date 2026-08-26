// Context Switcher Modal Component
// F1.15 — Organization / Branch / Register Operational Context Switcher

import React, { useState, useEffect, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { useShell } from '../../context/ShellContext'
import { getContextApi } from '../../services/contextApi'
import { validateContextHierarchy } from '../../context/contextSwitching'
import type { Organization } from '../../types/organization'
import type { Branch } from '../../types/branch'
import type { Register } from '../../types/register'

export interface ContextSwitcherModalProps {
  isOpen: boolean
  onClose: () => void
}

export const ContextSwitcherModal: React.FC<ContextSwitcherModalProps> = ({ isOpen, onClose }) => {
  const { t } = useTranslation()
  const { organization, branch, register, switchContext } = useShell()

  const [orgs, setOrgs] = useState<Organization[]>([])
  const [branches, setBranches] = useState<Branch[]>([])
  const [registers, setRegisters] = useState<Register[]>([])

  const [selectedOrgId, setSelectedOrgId] = useState<string>(organization?.id || '')
  const [selectedBranchId, setSelectedBranchId] = useState<string>(branch?.id || '')
  const [selectedRegisterId, setSelectedRegisterId] = useState<string>(register?.id || '')

  const [isLoadingOrgs, setIsLoadingOrgs] = useState<boolean>(false)
  const [isLoadingBranches, setIsLoadingBranches] = useState<boolean>(false)
  const [isLoadingRegisters, setIsLoadingRegisters] = useState<boolean>(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false)

  const modalRef = useRef<HTMLDivElement>(null)
  const previousActiveElement = useRef<HTMLElement | null>(null)

  // Initialize and load organizations when modal opens
  useEffect(() => {
    if (!isOpen) return

    previousActiveElement.current = document.activeElement as HTMLElement | null
    setSelectedOrgId(organization?.id || '')
    setSelectedBranchId(branch?.id || '')
    setSelectedRegisterId(register?.id || '')
    setErrorMessage(null)

    let isMounted = true
    setIsLoadingOrgs(true)

    async function loadInitialData() {
      try {
        const api = getContextApi()
        const fetchedOrgs = await api.listOrganizations()
        if (!isMounted) return
        setOrgs(fetchedOrgs)

        let activeOrgId = ''
        if (organization?.id) {
          activeOrgId = organization.id
        } else if (fetchedOrgs.length > 0) {
          activeOrgId = fetchedOrgs[0].id
        }

        if (activeOrgId) {
          setSelectedOrgId(activeOrgId)
          setIsLoadingBranches(true)
          const fetchedBranches = await api.listBranches(activeOrgId)
          if (!isMounted) return
          setBranches(fetchedBranches)

          let activeBranchId = ''
          if (branch?.organization_id === activeOrgId) {
            activeBranchId = branch.id
          } else if (fetchedBranches.length > 0) {
            activeBranchId = fetchedBranches[0].id
          }

          if (activeBranchId) {
            setSelectedBranchId(activeBranchId)
            setIsLoadingRegisters(true)
            const fetchedRegisters = await api.listRegisters(activeBranchId)
            if (!isMounted) return
            setRegisters(fetchedRegisters)

            let activeRegId = ''
            if (register?.branch_id === activeBranchId) {
              activeRegId = register.id
            } else if (fetchedRegisters.length > 0) {
              activeRegId = fetchedRegisters[0].id
            }
            setSelectedRegisterId(activeRegId)
          } else {
            setSelectedBranchId('')
            setSelectedRegisterId('')
            setRegisters([])
          }
        }
      } catch (err) {
        if (!isMounted) return
        setErrorMessage(
          err instanceof Error && err.message
            ? err.message
            : t('contextSwitcher.errors.loadFailed'),
        )
      } finally {
        if (isMounted) {
          setIsLoadingOrgs(false)
          setIsLoadingBranches(false)
          setIsLoadingRegisters(false)
        }
      }
    }

    void loadInitialData()

    return () => {
      isMounted = false
      if (previousActiveElement.current) {
        previousActiveElement.current.focus()
      }
    }
  }, [isOpen, organization, branch, register, t])

  // Handle Escape key to close
  useEffect(() => {
    if (!isOpen) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        onClose()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [isOpen, onClose])

  // Handle Organization change: fetch branches and invalidate stale branch/register
  const handleOrgChange = useCallback(
    async (newOrgId: string) => {
      setSelectedOrgId(newOrgId)
      setSelectedBranchId('')
      setSelectedRegisterId('')
      setBranches([])
      setRegisters([])
      setErrorMessage(null)

      if (!newOrgId) return

      setIsLoadingBranches(true)
      try {
        const api = getContextApi()
        const fetchedBranches = await api.listBranches(newOrgId)
        setBranches(fetchedBranches)
      } catch (err) {
        setErrorMessage(
          err instanceof Error && err.message
            ? err.message
            : t('contextSwitcher.errors.loadFailed'),
        )
      } finally {
        setIsLoadingBranches(false)
      }
    },
    [t],
  )

  // Handle Branch change: fetch registers and invalidate stale register
  const handleBranchChange = useCallback(
    async (newBranchId: string) => {
      setSelectedBranchId(newBranchId)
      setSelectedRegisterId('')
      setRegisters([])
      setErrorMessage(null)

      if (!newBranchId) return

      setIsLoadingRegisters(true)
      try {
        const api = getContextApi()
        const fetchedRegisters = await api.listRegisters(newBranchId)
        setRegisters(fetchedRegisters)
      } catch (err) {
        setErrorMessage(
          err instanceof Error && err.message
            ? err.message
            : t('contextSwitcher.errors.loadFailed'),
        )
      } finally {
        setIsLoadingRegisters(false)
      }
    },
    [t],
  )

  // Handle Register change
  const handleRegisterChange = useCallback((newRegisterId: string) => {
    setSelectedRegisterId(newRegisterId)
    setErrorMessage(null)
  }, [])

  // Handle Form Submit: Validate hierarchy and atomically switch ShellContext
  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault()
      if (isSubmitting) return

      const chosenOrg = orgs.find((o) => o.id === selectedOrgId) || null
      const chosenBranch = branches.find((b) => b.id === selectedBranchId) || null
      const chosenRegister = registers.find((r) => r.id === selectedRegisterId) || null

      if (!validateContextHierarchy(chosenOrg, chosenBranch, chosenRegister)) {
        setErrorMessage(t('contextSwitcher.errors.invalidHierarchy'))
        return
      }

      setIsSubmitting(true)
      try {
        // Enforce atomic context switch in ShellContext
        switchContext(chosenOrg!, chosenBranch!, chosenRegister!)
        onClose()
      } catch (err) {
        setErrorMessage(
          err instanceof Error && err.message
            ? err.message
            : t('contextSwitcher.errors.loadFailed'),
        )
      } finally {
        setIsSubmitting(false)
      }
    },
    [
      isSubmitting,
      orgs,
      branches,
      registers,
      selectedOrgId,
      selectedBranchId,
      selectedRegisterId,
      switchContext,
      onClose,
      t,
    ],
  )

  if (!isOpen) return null

  const isFormValid = Boolean(
    selectedOrgId &&
      selectedBranchId &&
      selectedRegisterId &&
      !isLoadingOrgs &&
      !isLoadingBranches &&
      !isLoadingRegisters,
  )

  return (
    <div
      className="context-modal-backdrop"
      role="presentation"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose()
      }}
      data-testid="context-switcher-backdrop"
    >
      <div
        ref={modalRef}
        className="context-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="context-switcher-title"
        data-testid="context-switcher-modal"
      >
        <header className="context-modal__header">
          <div className="context-modal__titles">
            <h2 id="context-switcher-title" className="context-modal__title">
              {t('contextSwitcher.title')}
            </h2>
            <p className="context-modal__subtitle">{t('contextSwitcher.subtitle')}</p>
          </div>
          <button
            type="button"
            className="context-modal__close-btn"
            onClick={onClose}
            aria-label={t('contextSwitcher.close')}
            data-testid="context-switcher-close-btn"
          >
            ✕
          </button>
        </header>

        {errorMessage && (
          <div role="alert" className="context-modal__error" data-testid="context-switcher-error">
            <span>{errorMessage}</span>
          </div>
        )}

        <form className="context-modal__form" onSubmit={handleSubmit} noValidate>
          {/* Organization Selection */}
          <div className="context-modal__group">
            <label htmlFor="context-select-org" className="context-modal__label">
              {t('contextSwitcher.organizationLabel')}
            </label>
            <select
              id="context-select-org"
              className="context-modal__select"
              value={selectedOrgId}
              onChange={(e) => void handleOrgChange(e.target.value)}
              disabled={isLoadingOrgs || isSubmitting}
              data-testid="context-select-org"
            >
              <option value="">{t('contextSwitcher.selectOrgPlaceholder')}</option>
              {orgs.map((org) => (
                <option key={org.id} value={org.id}>
                  {org.name} ({org.default_currency})
                </option>
              ))}
            </select>
            {orgs.length === 0 && !isLoadingOrgs && (
              <span className="context-modal__hint">{t('contextSwitcher.noOrganizations')}</span>
            )}
          </div>

          {/* Branch Selection */}
          <div className="context-modal__group">
            <label htmlFor="context-select-branch" className="context-modal__label">
              {t('contextSwitcher.branchLabel')}
            </label>
            <select
              id="context-select-branch"
              className="context-modal__select"
              value={selectedBranchId}
              onChange={(e) => void handleBranchChange(e.target.value)}
              disabled={isLoadingBranches || !selectedOrgId || isSubmitting}
              data-testid="context-select-branch"
            >
              <option value="">{t('contextSwitcher.selectBranchPlaceholder')}</option>
              {branches.map((b) => (
                <option key={b.id} value={b.id}>
                  {b.name}
                </option>
              ))}
            </select>
            {selectedOrgId && branches.length === 0 && !isLoadingBranches && (
              <span className="context-modal__hint">{t('contextSwitcher.noBranches')}</span>
            )}
          </div>

          {/* Register Selection */}
          <div className="context-modal__group">
            <label htmlFor="context-select-register" className="context-modal__label">
              {t('contextSwitcher.registerLabel')}
            </label>
            <select
              id="context-select-register"
              className="context-modal__select"
              value={selectedRegisterId}
              onChange={(e) => handleRegisterChange(e.target.value)}
              disabled={isLoadingRegisters || !selectedBranchId || isSubmitting}
              data-testid="context-select-register"
            >
              <option value="">{t('contextSwitcher.selectRegisterPlaceholder')}</option>
              {registers.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.name} {r.code ? `(${r.code})` : ''}
                </option>
              ))}
            </select>
            {selectedBranchId && registers.length === 0 && !isLoadingRegisters && (
              <span className="context-modal__hint">{t('contextSwitcher.noRegisters')}</span>
            )}
          </div>

          {/* Actions */}
          <div className="context-modal__actions">
            <button
              type="button"
              className="btn btn--secondary"
              onClick={onClose}
              disabled={isSubmitting}
              data-testid="context-switcher-cancel-btn"
            >
              {t('contextSwitcher.cancel')}
            </button>
            <button
              type="submit"
              className="btn btn--primary"
              disabled={!isFormValid || isSubmitting}
              data-testid="context-switcher-apply-btn"
            >
              {t('contextSwitcher.apply')}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
