// User Management View Component
// F1.16 — Roles / Permissions Administration UI

import React, { useState, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { getPermissionApi } from '../../services/permissionApi'
import {
  AUTHORITATIVE_ROLES,
  AUTHORITATIVE_PERMISSIONS,
  PERMISSION_CATALOG,
  computeEffectivePermissions,
  getRoleDefaultPermissions,
} from '../../context/permissionEvaluation'
import type { Role, Permission, PermissionEffect, UserPermissionOverride } from '../../types/permission'
import type { User } from '../../types/user'
import { CreateUserModal } from './CreateUserModal'

export interface UserManagementViewProps {
  branchId: string
  users: User[]
  onUsersChange: (users: User[]) => void
}

export const UserManagementView: React.FC<UserManagementViewProps> = ({
  branchId,
  users,
  onUsersChange,
}) => {
  const { t } = useTranslation()

  const [searchQuery, setSearchQuery] = useState('')
  const [selectedUser, setSelectedUser] = useState<User | null>(null)
  const [userOverrides, setUserOverrides] = useState<UserPermissionOverride[]>([])
  const [isLoadingOverrides, setIsLoadingOverrides] = useState(false)
  const [isUpdatingUser, setIsUpdatingUser] = useState(false)
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false)
  const [actionError, setActionError] = useState<string | null>(null)
  const [actionSuccess, setActionSuccess] = useState<string | null>(null)

  // Filter users by search query
  const filteredUsers = useMemo(() => {
    const q = searchQuery.trim().toLowerCase()
    if (!q) return users
    return users.filter(
      (u) =>
        u.full_name.toLowerCase().includes(q) ||
        (u.username && u.username.toLowerCase().includes(q)) ||
        u.role.toLowerCase().includes(q),
    )
  }, [users, searchQuery])

  // Select user and load their permission overrides
  const handleSelectUser = useCallback(
    async (user: User) => {
      setSelectedUser(user)
      setActionError(null)
      setActionSuccess(null)
      setIsLoadingOverrides(true)

      try {
        const api = getPermissionApi()
        const overrides = await api.listUserPermissionOverrides(user.id)
        setUserOverrides(overrides)
      } catch (err) {
        setActionError(
          err instanceof Error && err.message
            ? err.message
            : t('admin.users.errors.loadOverridesFailed'),
        )
      } finally {
        setIsLoadingOverrides(false)
      }
    },
    [t],
  )

  // Handle Quick Role Change in table or drawer
  const handleRoleChange = useCallback(
    async (userId: string, newRole: Role) => {
      setIsUpdatingUser(true)
      setActionError(null)
      setActionSuccess(null)

      try {
        const api = getPermissionApi()
        const updated = await api.updateUser(userId, { role: newRole })

        const updatedUsers = users.map((u) => (u.id === userId ? updated : u))
        onUsersChange(updatedUsers)

        if (selectedUser?.id === userId) {
          setSelectedUser(updated)
        }
        setActionSuccess(t('admin.users.success.roleUpdated'))
      } catch (err) {
        setActionError(
          err instanceof Error && err.message
            ? err.message
            : t('admin.users.errors.updateRoleFailed'),
        )
      } finally {
        setIsUpdatingUser(false)
      }
    },
    [users, selectedUser, onUsersChange, t],
  )

  // Handle Active/Inactive Status Toggle
  const handleToggleActive = useCallback(
    async (user: User) => {
      setIsUpdatingUser(true)
      setActionError(null)
      setActionSuccess(null)

      try {
        const api = getPermissionApi()
        const updated = await api.updateUser(user.id, { is_active: !user.is_active })

        const updatedUsers = users.map((u) => (u.id === user.id ? updated : u))
        onUsersChange(updatedUsers)

        if (selectedUser?.id === user.id) {
          setSelectedUser(updated)
        }
        setActionSuccess(t('admin.users.success.statusUpdated'))
      } catch (err) {
        setActionError(
          err instanceof Error && err.message
            ? err.message
            : t('admin.users.errors.updateStatusFailed'),
        )
      } finally {
        setIsUpdatingUser(false)
      }
    },
    [users, selectedUser, onUsersChange, t],
  )

  // Handle Permission Override Toggle (Allow, Deny, Default)
  const handleOverrideChange = useCallback(
    async (permission: Permission, newEffect: PermissionEffect | 'default') => {
      if (!selectedUser) return
      setIsUpdatingUser(true)
      setActionError(null)
      setActionSuccess(null)

      try {
        const api = getPermissionApi()
        if (newEffect === 'default') {
          await api.removeUserPermissionOverride(selectedUser.id, permission)
          setUserOverrides((prev) => prev.filter((o) => o.permission !== permission))
        } else {
          await api.setUserPermissionOverride(selectedUser.id, permission, newEffect)
          setUserOverrides((prev) => {
            const next = prev.filter((o) => o.permission !== permission)
            next.push({ permission, effect: newEffect })
            return next
          })
        }
        setActionSuccess(t('admin.users.success.overrideUpdated'))
      } catch (err) {
        setActionError(
          err instanceof Error && err.message
            ? err.message
            : t('admin.users.errors.updateOverrideFailed'),
        )
      } finally {
        setIsUpdatingUser(false)
      }
    },
    [selectedUser, t],
  )

  const handleUserCreated = useCallback(
    (newUser: User) => {
      onUsersChange([newUser, ...users])
      setActionSuccess(t('admin.users.success.userCreated'))
      void handleSelectUser(newUser)
    },
    [users, onUsersChange, handleSelectUser, t],
  )

  // Compute effective permissions for selected user
  const effectivePermissions = useMemo(() => {
    if (!selectedUser) return []
    return computeEffectivePermissions(selectedUser.role, userOverrides)
  }, [selectedUser, userOverrides])

  return (
    <div className="admin-users-view" data-testid="user-management-view">
      {/* Top Action Bar */}
      <div className="admin-toolbar">
        <div className="admin-search-box">
          <input
            type="search"
            className="form-input admin-search-input"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t('admin.users.searchPlaceholder')}
            aria-label={t('admin.users.searchPlaceholder')}
            data-testid="users-search-input"
          />
        </div>
        <button
          type="button"
          className="btn btn--primary admin-add-user-btn"
          onClick={() => setIsCreateModalOpen(true)}
          data-testid="open-create-user-modal-btn"
        >
          + {t('admin.users.addUserBtn')}
        </button>
      </div>

      {/* Notifications */}
      {actionError && (
        <div role="alert" className="context-modal__error admin-alert" data-testid="admin-action-error">
          <span>{actionError}</span>
        </div>
      )}
      {actionSuccess && (
        <div role="status" className="admin-alert admin-alert--success" data-testid="admin-action-success">
          <span>{actionSuccess}</span>
        </div>
      )}

      {/* Main Content Layout: Table + Detail Panel */}
      <div className="admin-users-layout">
        <div className="admin-users-table-pane">
          {filteredUsers.length === 0 ? (
            <div className="admin-empty-table" data-testid="users-empty-state">
              <p>{searchQuery ? t('admin.users.noSearchResults') : t('admin.users.noUsersInBranch')}</p>
            </div>
          ) : (
            <div className="admin-table-container">
              <table className="admin-table" aria-label={t('admin.users.tableAriaLabel')}>
                <thead>
                  <tr>
                    <th scope="col">{t('admin.users.headers.user')}</th>
                    <th scope="col">{t('admin.users.headers.role')}</th>
                    <th scope="col">{t('admin.users.headers.status')}</th>
                    <th scope="col">{t('admin.users.headers.actions')}</th>
                  </tr>
                </thead>
                <tbody>
                  {filteredUsers.map((user) => {
                    const isSelected = selectedUser?.id === user.id
                    return (
                      <tr
                        key={user.id}
                        className={`admin-user-row ${isSelected ? 'admin-user-row--selected' : ''}`}
                        onClick={() => void handleSelectUser(user)}
                        tabIndex={0}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter' || e.key === ' ') {
                            e.preventDefault()
                            void handleSelectUser(user)
                          }
                        }}
                        data-testid={`user-row-${user.id}`}
                      >
                        <td className="user-info-cell">
                          <div className="user-name">{user.full_name}</div>
                          {user.username && <div className="user-handle">@{user.username}</div>}
                        </td>
                        <td>
                          <select
                            className="admin-role-select"
                            value={user.role}
                            onClick={(e) => e.stopPropagation()}
                            onChange={(e) => void handleRoleChange(user.id, e.target.value as Role)}
                            disabled={isUpdatingUser}
                            aria-label={`${user.full_name} ${t('admin.users.fields.role')}`}
                            data-testid={`role-select-${user.id}`}
                          >
                            {AUTHORITATIVE_ROLES.map((r) => (
                              <option key={r} value={r}>
                                {t(`roles.${r}.title`)}
                              </option>
                            ))}
                          </select>
                        </td>
                        <td>
                          <button
                            type="button"
                            className={`status-badge ${user.is_active ? 'status-badge--active' : 'status-badge--inactive'}`}
                            onClick={(e) => {
                              e.stopPropagation()
                              void handleToggleActive(user)
                            }}
                            disabled={isUpdatingUser}
                            aria-label={`${user.full_name}: ${user.is_active ? t('common.active') : t('common.inactive')}`}
                            data-testid={`toggle-status-${user.id}`}
                          >
                            {user.is_active ? t('common.active') : t('common.inactive')}
                          </button>
                        </td>
                        <td>
                          <button
                            type="button"
                            className="btn btn--secondary btn--sm"
                            onClick={(e) => {
                              e.stopPropagation()
                              void handleSelectUser(user)
                            }}
                            data-testid={`inspect-user-${user.id}`}
                          >
                            {t('admin.users.inspectBtn')}
                          </button>
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* User Detail & Permission Override Drawer */}
        {selectedUser && (
          <aside className="admin-user-detail-pane" data-testid="user-detail-pane">
            <div className="admin-detail-header">
              <div>
                <h3 className="admin-detail-title">{selectedUser.full_name}</h3>
                <span className="admin-detail-subtitle">
                  {selectedUser.username ? `@${selectedUser.username}` : t('admin.users.localUser')}
                </span>
              </div>
              <button
                type="button"
                className="context-modal__close-btn"
                onClick={() => setSelectedUser(null)}
                aria-label={t('common.close')}
                data-testid="close-detail-pane-btn"
              >
                ✕
              </button>
            </div>

            <div className="admin-detail-meta">
              <div className="meta-item">
                <span className="meta-label">{t('admin.users.fields.role')}:</span>
                <span className={`role-badge role-badge--${selectedUser.role}`}>
                  {t(`roles.${selectedUser.role}.title`)}
                </span>
              </div>
              <div className="meta-item">
                <span className="meta-label">{t('admin.users.effectiveCount')}:</span>
                <span className="meta-value">{effectivePermissions.length} / 17</span>
              </div>
            </div>

            {isLoadingOverrides ? (
              <div className="admin-loading-overrides">{t('common.loading')}</div>
            ) : (
              <div className="admin-overrides-section">
                <h4 className="admin-overrides-title">{t('admin.users.permissionOverridesTitle')}</h4>
                <p className="admin-overrides-subtitle">{t('admin.users.permissionOverridesSubtitle')}</p>

                <div className="admin-overrides-list">
                  {PERMISSION_CATALOG.map((perm) => {
                    const defaultHas = getRoleDefaultPermissions(selectedUser.role).includes(perm.code as Permission)
                    const override = userOverrides.find((o) => o.permission === perm.code)
                    const currentEffect = override ? override.effect : 'default'
                    const isAllowed = effectivePermissions.includes(perm.code as Permission)

                    return (
                      <div key={perm.code} className="override-row" data-testid={`override-row-${perm.code}`}>
                        <div className="override-info">
                          <span className="override-name">{t(perm.titleKey)}</span>
                          <code className="override-code">{perm.code}</code>
                        </div>
                        <div className="override-controls">
                          <span
                            className={`status-indicator ${isAllowed ? 'status-indicator--allowed' : 'status-indicator--denied'}`}
                          >
                            {isAllowed ? '✓' : '✕'}
                          </span>
                          <select
                            className="override-select"
                            value={currentEffect}
                            onChange={(e) =>
                              void handleOverrideChange(
                                perm.code as Permission,
                                e.target.value as PermissionEffect | 'default',
                              )
                            }
                            disabled={isUpdatingUser}
                            aria-label={`${t(perm.titleKey)} ${t('admin.users.overrideLabel')}`}
                            data-testid={`override-select-${perm.code}`}
                          >
                            <option value="default">
                              {t('admin.users.overrideDefault')} ({defaultHas ? t('common.allow') : t('common.deny')})
                            </option>
                            <option value="allow">{t('admin.users.overrideAllow')}</option>
                            <option value="deny">{t('admin.users.overrideDeny')}</option>
                          </select>
                        </div>
                      </div>
                    )
                  })}
                </div>
              </div>
            )}
          </aside>
        )}
      </div>

      {/* Create User Modal */}
      <CreateUserModal
        isOpen={isCreateModalOpen}
        branchId={branchId}
        onClose={() => setIsCreateModalOpen(false)}
        onUserCreated={handleUserCreated}
      />
    </div>
  )
}
