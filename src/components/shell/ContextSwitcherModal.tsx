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
} from '../../context/contextSwitching'
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
  const activeOrgId = currentOrg?.id || (fetchedOrgs[0]?.id ?? '')
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

  // Request sequencing refs to avoid race conditions
  const orgReqSeq = useRef<number>(0)
  const branchReqSeq = useRef<number>(0)

  // Focus management: track active element on open and restore exclusively on close
  useEffect(() => {
    if (isOpen) {
      previousActiveElement.current = document.activeElement as HTMLElement | null
    } else if (previousActiveElement.current) {
      previousActiveElement.current.focus()
      previousActiveElement.current = null
    }
  }, [isOpen])

  // Handle Escape key and outside click to close
  useEffect(() => {
    if (!isOpen) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        onClose()
      }
    }

    const handleOutsideClick = (e: MouseEvent) => {
      if (modalRef.current && !modalRef.current.contains(e.target as Node)) {
        onClose()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    document.addEventListener('mousedown', handleOutsideClick)

    return () => {
      window.removeEventListener('keydown', handleKeyDown)
      document.removeEventListener('mousedown', handleOutsideClick)
    }
  }, [isOpen, onClose])

  // Load registers for a specific branch with sequencing guard
  const loadRegistersForBranch = useCallback(
    async (orgId: string, branchId: string) => {
      const seq = ++branchReqSeq.current
      setIsLoadingRegisters(true)

      try {
        const api = getContextApi()
        const fetchedRegisters = await api.listRegisters(branchId)
        if (seq !== branchReqSeq.current) return

        setRegisters(fetchedRegisters)
        const preservedReg = resolveRegisterOnBranchChange(orgId, branchId, register, fetchedRegisters)
        if (preservedReg) {
          setSelectedRegisterId(preservedReg.id)
        } else {
          setSelectedRegisterId('')
        }
      } catch (err) {
        if (seq === branchReqSeq.current) {
          setErrorMessage(
            err instanceof Error && err.message ? err.message : t('contextSwitcher.errors.loadFailed'),
          )
        }
      } finally {
        if (seq === branchReqSeq.current) {
          setIsLoadingRegisters(false)
        }
      }
    },
    [register, t],
  )

  // Handle Organization change: fetch branches and invalidate stale branch/register
  const handleOrgChange = useCallback(
    async (newOrgId: string) => {
      const seq = ++orgReqSeq.current
      ++branchReqSeq.current // Invalidate any in-flight branch registers request

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
        if (seq !== orgReqSeq.current) return

        setBranches(fetchedBranches)
        const preservedBranch = resolveBranchOnOrgChange(newOrgId, branch, fetchedBranches)
        if (preservedBranch) {
          setSelectedBranchId(preservedBranch.id)
          void loadRegistersForBranch(newOrgId, preservedBranch.id)
        }
      } catch (err) {
        if (seq === orgReqSeq.current) {
          setErrorMessage(
            err instanceof Error && err.message ? err.message : t('contextSwitcher.errors.loadFailed'),
          )
        }
      } finally {
        if (seq === orgReqSeq.current) {
          setIsLoadingBranches(false)
        }
      }
    },
    [branch, loadRegistersForBranch, t],
  )

  // Handle Branch change: fetch registers and invalidate stale register
  const handleBranchChange = useCallback(
    async (newBranchId: string) => {
      setSelectedBranchId(newBranchId)
      setSelectedRegisterId('')
      setRegisters([])
      setErrorMessage(null)

      if (!newBranchId) {
        ++branchReqSeq.current
        return
      }

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
        open
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
      </dialog>
    </div>
  )
}
