// Roles / Permissions Administration Main Component
// F1.16 — Roles / Permissions Administration UI

import React, { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useShell } from '../../context/ShellContext'
import { useAuth } from '../../context/AuthContext'
import { getPermissionApi } from '../../services/permissionApi'
import { hasEffectivePermission } from '../../context/permissionEvaluation'
import type { User } from '../../types/user'
import { UserManagementView } from './UserManagementView'
import { RoleMatrixView } from './RoleMatrixView'
import { PermissionDeniedState } from '../common/PermissionDeniedState'
import { LoadingSkeleton } from '../common/LoadingSkeleton'
import { ErrorState } from '../common/ErrorState'

export type AdminTab = 'users' | 'matrix'

export const RolesPermissionsAdmin: React.FC = () => {
  const { t } = useTranslation()
  const { branch, setActiveRoute } = useShell()
  const { activeUser } = useAuth()

  const [activeTab, setActiveTab] = useState<AdminTab>('users')
  const [users, setUsers] = useState<User[]>([])
  const [isLoading, setIsLoading] = useState<boolean>(true)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

  // Verify that active user is authorized to manage users & permissions
  // Presentation gating only: Rust backend remains authoritative
  const isAuthorized = Boolean(
    activeUser?.role &&
      hasEffectivePermission(activeUser.role, [], 'users.manage'),
  )

  const loadBranchUsers = useCallback(async () => {
    if (!branch?.id) return
    setIsLoading(true)
    setErrorMessage(null)

    try {
      const api = getPermissionApi()
      const fetched = await api.listUsers(branch.id)
      setUsers(fetched)
    } catch (err) {
      setErrorMessage(
        err instanceof Error && err.message ? err.message : t('admin.users.errors.loadUsersFailed'),
      )
    } finally {
      setIsLoading(false)
    }
  }, [branch?.id, t])

  useEffect(() => {
    if (isAuthorized && branch?.id) {
      void loadBranchUsers()
    } else {
      setIsLoading(false)
    }
  }, [isAuthorized, branch?.id, loadBranchUsers])

  // Unauthorized Access Gate
  if (!isAuthorized) {
    return (
      <div className="admin-permission-denied-wrapper" data-testid="admin-permission-denied">
        <PermissionDeniedState
          permission="users.manage"
          onAction={() => setActiveRoute('pos')}
        />
      </div>
    )
  }

  // Loading State
  if (isLoading) {
    return (
      <div className="admin-loading-wrapper" data-testid="admin-loading">
        <LoadingSkeleton cardsCount={4} />
      </div>
    )
  }

  // Error State
  if (errorMessage) {
    return (
      <div className="admin-error-wrapper" data-testid="admin-error">
        <ErrorState
          message={errorMessage}
          onRetry={() => void loadBranchUsers()}
        />
      </div>
    )
  }

  return (
    <div className="admin-container" data-testid="roles-permissions-admin">
      {/* Navigation Tabs */}
      <div className="admin-tabs" role="tablist" aria-label={t('admin.tabs.ariaLabel')}>
        <button
          type="button"
          role="tab"
          id="tab-users"
          aria-controls="panel-users"
          aria-selected={activeTab === 'users'}
          className={`admin-tab ${activeTab === 'users' ? 'admin-tab--active' : ''}`}
          onClick={() => setActiveTab('users')}
          data-testid="tab-users-btn"
        >
          {t('admin.tabs.users')} ({users.length})
        </button>
        <button
          type="button"
          role="tab"
          id="tab-matrix"
          aria-controls="panel-matrix"
          aria-selected={activeTab === 'matrix'}
          className={`admin-tab ${activeTab === 'matrix' ? 'admin-tab--active' : ''}`}
          onClick={() => setActiveTab('matrix')}
          data-testid="tab-matrix-btn"
        >
          {t('admin.tabs.matrix')}
        </button>
      </div>

      {/* Tab Panels */}
      <div
        id="panel-users"
        role="tabpanel"
        aria-labelledby="tab-users"
        hidden={activeTab !== 'users'}
        className="admin-tabpanel"
      >
        {activeTab === 'users' && (
          <UserManagementView
            branchId={branch?.id || ''}
            users={users}
            onUsersChange={setUsers}
          />
        )}
      </div>

      <div
        id="panel-matrix"
        role="tabpanel"
        aria-labelledby="tab-matrix"
        hidden={activeTab !== 'matrix'}
        className="admin-tabpanel"
      >
        {activeTab === 'matrix' && <RoleMatrixView />}
      </div>
    </div>
  )
}
