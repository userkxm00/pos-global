// Role Matrix View Component
// F1.16 — Roles / Permissions Administration UI

import React from 'react'
import { useTranslation } from 'react-i18next'
import {
  AUTHORITATIVE_PERMISSIONS,
  AUTHORITATIVE_ROLES,
  CATEGORY_ORDER,
  PERMISSION_CATALOG,
  ROLE_CATALOG,
  ROLE_DEFAULT_PERMISSIONS,
} from '../../context/permissionEvaluation'
import type { PermissionCategory, Permission } from '../../types/permission'

export const RoleMatrixView: React.FC = () => {
  const { t } = useTranslation()
  const totalCount = AUTHORITATIVE_PERMISSIONS.length

  return (
    <div className="admin-matrix-view" data-testid="role-matrix-view">
      <div className="admin-matrix-header">
        <h2 className="admin-matrix-title">{t('admin.matrix.title')}</h2>
        <p className="admin-matrix-subtitle">{t('admin.matrix.subtitle')}</p>
      </div>

      {/* Role Summary Cards */}
      <div className="role-summary-cards">
        {ROLE_CATALOG.map((r) => (
          <div key={r.role} className={`role-summary-card role-summary-card--${r.role}`}>
            <div className="role-summary-card__header">
              <span className={`role-badge role-badge--${r.role}`}>
                {t(r.titleKey)}
              </span>
              <span className="role-summary-card__count">
                {r.defaultPermissions.length} / {totalCount} {t('admin.matrix.permissions')}
              </span>
            </div>
            <p className="role-summary-card__desc">{t(r.descriptionKey)}</p>
          </div>
        ))}
      </div>

      {/* Matrix Table */}
      <div className="admin-table-container">
        <table className="admin-table admin-matrix-table" aria-label={t('admin.matrix.tableAriaLabel')}>
          <thead>
            <tr>
              <th scope="col" className="matrix-col-permission">
                {t('admin.matrix.permissionHeader')}
              </th>
              {AUTHORITATIVE_ROLES.map((role) => (
                <th key={role} scope="col" className="matrix-col-role">
                  {t(`roles.${role}.title`)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {CATEGORY_ORDER.map((category: PermissionCategory) => {
              const categoryPermissions = PERMISSION_CATALOG.filter((p) => p.category === category)
              if (categoryPermissions.length === 0) return null

              return (
                <React.Fragment key={category}>
                  <tr className="matrix-category-row">
                    <th
                      colSpan={AUTHORITATIVE_ROLES.length + 1}
                      scope="colgroup"
                      className="matrix-category-header"
                    >
                      {t(`permissions.categories.${category}`)}
                    </th>
                  </tr>
                  {categoryPermissions.map((perm) => (
                    <tr key={perm.code} className="matrix-perm-row">
                      <td className="matrix-perm-info">
                        <span className="matrix-perm-title">{t(perm.titleKey)}</span>
                        <code className="matrix-perm-code">{perm.code}</code>
                        <span className="matrix-perm-desc">{t(perm.descriptionKey)}</span>
                      </td>
                      {AUTHORITATIVE_ROLES.map((role) => {
                        const hasPerm = ROLE_DEFAULT_PERMISSIONS[role].includes(perm.code as Permission)
                        const roleTitle = t(`roles.${role}.title`)
                        const permTitle = t(perm.titleKey)
                        const statusTitle = hasPerm ? t('common.allowed') : t('common.denied')
                        const cellAriaLabel = `${permTitle} - ${roleTitle}: ${statusTitle}`

                        return (
                          <td key={`${role}-${perm.code}`} className="matrix-cell-status">
                            {hasPerm ? (
                              <span
                                className="status-icon status-icon--allowed"
                                aria-label={cellAriaLabel}
                                title={cellAriaLabel}
                              >
                                ✓
                              </span>
                            ) : (
                              <span
                                className="status-icon status-icon--denied"
                                aria-label={cellAriaLabel}
                                title={cellAriaLabel}
                              >
                                —
                              </span>
                            )}
                          </td>
                        )
                      })}
                    </tr>
                  ))}
                </React.Fragment>
              )
            })}
          </tbody>
        </table>
      </div>
    </div>
  )
}
