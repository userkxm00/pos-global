// Context Switcher Modal Component
// F1.15 — Organization / Branch / Register Operational Context Switcher

import React, { useState, useEffect, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { useShell } from '../../context/ShellContext'
import { getContextApi } from '../../services/contextApi'
import type { ContextApiClient } from '../../services/contextApi'
import {
  validateContextHierarchy,
  resolveBranchOnOrgChange,
  resolveRegisterOnBranchChange,
  createCascadingSequenceGuard,
  performGuardedOrgFetch,
  performGuardedBranchFetch,
} from '../../context/contextSwitching'
import type { CascadingSequenceGuard } from '../../context/contextSwitching'
import type { Organization } from '../../types/organization'
import type { Branch } from '../../types/branch'
import type { Register } from '../../types/register'

export interface ContextSwitcherModalProps {
  isOpen: boolean
  onClose: () => void
}

interface InitialContextResult {
  orgs: Organization[]
  selectedOrgId: string
  branches: Branch[]
  selectedBranchId: string
  registers: Register[]
  selectedRegisterId: string
}

async function fetchInitialOrgContext(
  api: ContextApiClient,
  currentOrg: Organization | null,
  currentBranch: Branch | null,
  currentRegister: Register | null,
): Promise<InitialContextResult> {
  const fetchedOrgs = await api.listOrganizations()
  const retainedOrg = currentOrg && fetchedOrgs.find((org) => org.id === currentOrg.id)
  const activeOrgId = retainedOrg?.id || (fetchedOrgs[0]?.id ?? '')
  if (!activeOrgId) {
    return {
      orgs: fetchedOrgs,
      selectedOrgId: '',
      branches: [],
      selectedBranchId: '',
      registers: [],
      selectedRegisterId: '',
    }
  }

  const fetchedBranches = await api.listBranches(activeOrgId)
  const initialBranch = resolveBranchOnOrgChange(activeOrgId, currentBranch, fetchedBranches)
  const activeBranchId = initialBranch?.id || (fetchedBranches[0]?.id ?? '')
  if (!activeBranchId) {
    return {
      orgs: fetchedOrgs,
      selectedOrgId: activeOrgId,
      branches: fetchedBranches,
      selectedBranchId: '',
      registers: [],
      selectedRegisterId: '',
    }
  }

  const fetchedRegisters = await api.listRegisters(activeBranchId)
  const initialReg = resolveRegisterOnBranchChange(
    activeOrgId,
    activeBranchId,
    currentRegister,
    fetchedRegisters,
  )
  const activeRegId = initialReg?.id || (fetchedRegisters[0]?.id ?? '')

  return {
    orgs: fetchedOrgs,
    selectedOrgId: activeOrgId,
    branches: fetchedBranches,
    selectedBranchId: activeBranchId,
    registers: fetchedRegisters,
    selectedRegisterId: activeRegId,
  }
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

  const modalRef = useRef<HTMLDialogElement>(null)
  const previousActiveElement = useRef<HTMLElement | null>(null)

  // Request sequencing guard to discard out-of-order race conditions
  const guardRef = useRef<CascadingSequenceGuard>(createCascadingSequenceGuard())

  // Native modal dialog lifecycle and focus restoration
  useEffect(() => {
    const dialog = modalRef.current
    if (!dialog) return

    if (isOpen) {
      previousActiveElement.current = document.activeElement as HTMLElement | null
      if (typeof dialog.showModal === 'function' && !dialog.open) {
        dialog.showModal()
      }
    }

    return () => {
      if (dialog && typeof dialog.close === 'function' && dialog.open) {
        dialog.close()
      }
      if (previousActiveElement.current) {
        previousActiveElement.current.focus()
        previousActiveElement.current = null
      }
    }
  }, [isOpen])

  // Load registers for a specific branch with sequence guard
  const loadRegistersForBranch = useCallback(
    async (orgId: string, branchId: string) => {
      setIsLoadingRegisters(true)
      const currentSeq = guardRef.current.branchReqSeq + 1

      try {
        const api = getContextApi()
        await performGuardedBranchFetch(
          (bId) => api.listRegisters(bId),
          branchId,
          guardRef.current,
          (fetchedRegisters, seq) => {
            if (seq === guardRef.current.branchReqSeq) {
              setRegisters(fetchedRegisters)
              const preservedReg = resolveRegisterOnBranchChange(
                orgId,
                branchId,
                register,
                fetchedRegisters,
              )
              setSelectedRegisterId(preservedReg ? preservedReg.id : '')
            }
          },
        )
      } catch (err) {
        if (currentSeq >= guardRef.current.branchReqSeq) {
          setErrorMessage(
            err instanceof Error && err.message ? err.message : t('contextSwitcher.errors.loadFailed'),
          )
        }
      } finally {
        if (currentSeq >= guardRef.current.branchReqSeq) {
          setIsLoadingRegisters(false)
        }
      }
    },
    [register, t],
  )

  // Handle Organization change: fetch branches with sequence guard
  const handleOrgChange = useCallback(
    async (newOrgId: string) => {
      setSelectedOrgId(newOrgId)
      setSelectedBranchId('')
      setSelectedRegisterId('')
      setBranches([])
      setRegisters([])
      setErrorMessage(null)

      setIsLoadingBranches(true)
      const currentSeq = guardRef.current.orgReqSeq + 1

      try {
        const api = getContextApi()
        await performGuardedOrgFetch(
          (oId) => api.listBranches(oId),
          newOrgId,
          guardRef.current,
          (fetchedBranches, seq) => {
            if (seq === guardRef.current.orgReqSeq) {
              setBranches(fetchedBranches)
              const preservedBranch = resolveBranchOnOrgChange(newOrgId, branch, fetchedBranches)
              if (preservedBranch) {
                setSelectedBranchId(preservedBranch.id)
                void loadRegistersForBranch(newOrgId, preservedBranch.id)
              }
            }
          },
        )
      } catch (err) {
        if (currentSeq >= guardRef.current.orgReqSeq) {
          setErrorMessage(
            err instanceof Error && err.message ? err.message : t('contextSwitcher.errors.loadFailed'),
          )
        }
      } finally {
        if (currentSeq >= guardRef.current.orgReqSeq) {
          setIsLoadingBranches(false)
        }
      }
    },
    [branch, loadRegistersForBranch, t],
  )

  // Handle Branch change: fetch registers with sequence guard
  const handleBranchChange = useCallback(
    async (newBranchId: string) => {
      setSelectedBranchId(newBranchId)
      setSelectedRegisterId('')
      setRegisters([])
      setErrorMessage(null)

      await loadRegistersForBranch(selectedOrgId, newBranchId)
    },
    [selectedOrgId, loadRegistersForBranch],
  )

  // Handle Register change
  const handleRegisterChange = useCallback((newRegisterId: string) => {
    setSelectedRegisterId(newRegisterId)
    setErrorMessage(null)
  }, [])

  // Initialize data on modal open
  useEffect(() => {
    if (!isOpen) return

    let isMounted = true
    setIsLoadingOrgs(true)
    setIsLoadingBranches(true)
    setIsLoadingRegisters(true)
    setErrorMessage(null)

    async function loadInitial() {
      try {
        const result = await fetchInitialOrgContext(
          getContextApi(),
          organization,
          branch,
          register,
        )
        if (!isMounted) return

        setOrgs(result.orgs)
        setSelectedOrgId(result.selectedOrgId)
        setBranches(result.branches)
        setSelectedBranchId(result.selectedBranchId)
        setRegisters(result.registers)
        setSelectedRegisterId(result.selectedRegisterId)
      } catch (err) {
        if (isMounted) {
          setErrorMessage(
            err instanceof Error && err.message ? err.message : t('contextSwitcher.errors.loadFailed'),
          )
        }
      } finally {
        if (isMounted) {
          setIsLoadingOrgs(false)
          setIsLoadingBranches(false)
          setIsLoadingRegisters(false)
        }
      }
    }

    void loadInitial()

    return () => {
      isMounted = false
    }
  }, [isOpen, organization, branch, register, t])

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
        switchContext(chosenOrg!, chosenBranch!, chosenRegister!)
        onClose()
      } catch (err) {
        setErrorMessage(
          err instanceof Error && err.message ? err.message : t('contextSwitcher.errors.loadFailed'),
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
    <div className="context-modal-backdrop" data-testid="context-switcher-backdrop">
      <dialog
        ref={modalRef}
        className="context-modal"
        aria-modal="true"
        aria-labelledby="context-switcher-title"
        onClick={(e) => {
          if (e.target === e.currentTarget) {
            onClose()
          }
        }}
        onCancel={(e) => {
          e.preventDefault()
          onClose()
        }}
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
      </dialog>
    </div>
  )
}
