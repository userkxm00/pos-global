// Main Content View Container with Authorization and Error States
// F1.11 — Shell & F1.18 — Authorization and Error-State UX

import React, { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useShell, NavigationRoute } from '../../context/ShellContext'
import { useAuth } from '../../context/AuthContext'
import type { Permission, UserPermissionOverride } from '../../types/permission'
import { getPermissionApi } from '../../services/permissionApi'
import { EMPTY_OVERRIDES } from '../../context/permissionEvaluation'
import { OfflineBanner } from '../common/OfflineBanner'
import { LoadingSkeleton } from '../common/LoadingSkeleton'
import { EmptyState } from '../common/EmptyState'
import { ErrorState } from '../common/ErrorState'
import { PermissionDeniedState } from '../common/PermissionDeniedState'
import { PermissionGate } from '../common/PermissionGate'
import { RolesPermissionsAdmin } from '../admin/RolesPermissionsAdmin'

export const ROUTE_PERMISSIONS: Record<NavigationRoute, Permission | undefined> = {
  pos: 'sales.create',
  shifts: 'cash.open',
  inventory: 'products.manage',
  customers: 'customers.manage',
  reports: 'reports.view',
  users: 'users.manage',
  tenants: 'settings.manage',
  settings: 'settings.manage',
}

export interface UserOverrideState {
  userId: string | null
  overrides: UserPermissionOverride[]
  isLoading: boolean
}

interface WorkspaceModuleViewProps {
  activeRoute: NavigationRoute
  routeTitle: string
  systemReadyText: string
}

const WorkspaceModuleView: React.FC<WorkspaceModuleViewProps> = ({
  activeRoute,
  routeTitle,
  systemReadyText,
}) => {
  if (activeRoute === 'users') {
    return <RolesPermissionsAdmin />
  }

  return (
    <div
      className="state-container"
      style={{ minHeight: '400px', justifyContent: 'flex-start', alignItems: 'stretch' }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span style={{ fontWeight: 'var(--font-weight-semibold)', color: 'var(--color-text-secondary)' }}>
          {systemReadyText} — {routeTitle}
        </span>
      </div>
      <div
        style={{
          marginBlockStart: 'var(--space-6)',
          padding: 'var(--space-4)',
          backgroundColor: 'var(--color-bg-surface-sunken)',
          borderRadius: 'var(--radius-md)',
          fontSize: 'var(--font-size-sm)',
          color: 'var(--color-text-secondary)',
          lineHeight: 'var(--line-height-relaxed)',
        }}
      >
        <p style={{ margin: 0 }}>
          <strong>Foundation Workspace</strong>: Module <code>{activeRoute}</code> active.
          Authorization, multi-tenant boundaries, and local SQLite data layers are verified and enforced.
        </p>
      </div>
    </div>
  )
}

export const MainContent: React.FC = () => {
  const { t } = useTranslation()
  const {
    activeRoute,
    setActiveRoute,
    isOnline,
    pendingSyncCount,
    viewState,
    setViewState,
    errorMessage,
    deniedPermission,
  } = useShell()

  const { activeUser } = useAuth()
  const [overrideState, setOverrideState] = useState<UserOverrideState>({
    userId: null,
    overrides: [],
    isLoading: false,
  })

  useEffect(() => {
    let isMounted = true
    const currentUserId = activeUser?.id ?? null

    if (!currentUserId) {
      setOverrideState({ userId: null, overrides: [], isLoading: false })
      return
    }

    // Immediately clear previous-user overrides and indicate loading on user change
    setOverrideState({ userId: currentUserId, overrides: [], isLoading: true })

    async function loadOverrides() {
      try {
        const api = getPermissionApi()
        const overrides = await api.listUserPermissionOverrides(currentUserId!)
        if (isMounted) {
          setOverrideState({ userId: currentUserId, overrides, isLoading: false })
        }
      } catch {
        if (isMounted) {
          // Fail-closed on error with empty overrides
          setOverrideState({ userId: currentUserId, overrides: [], isLoading: false })
        }
      }
    }

    void loadOverrides()

    return () => {
      isMounted = false
    }
  }, [activeUser?.id])

  const handleReturnToSafeRoute = useCallback(() => {
    setActiveRoute('pos')
    setViewState('idle')
  }, [setActiveRoute, setViewState])

  const routeTitleKey = `nav.items.${activeRoute}`
  const routeTitle = t(routeTitleKey)
  const requiredPermission = ROUTE_PERMISSIONS[activeRoute]

  const isOverrideForActiveUser = overrideState.userId === activeUser?.id
  const effectiveOverrides = isOverrideForActiveUser ? overrideState.overrides : EMPTY_OVERRIDES
  const isAuthHydrating = requiredPermission && overrideState.isLoading

  const renderDynamicContent = () => {
    if (viewState === 'loading' || isAuthHydrating) {
      return <LoadingSkeleton />
    }

    if (viewState === 'empty') {
      return <EmptyState onAction={() => setViewState('idle')} />
    }

    if (viewState === 'error') {
      return <ErrorState message={errorMessage} onRetry={() => setViewState('idle')} />
    }

    if (viewState === 'permission-denied') {
      return (
        <PermissionDeniedState
          permission={deniedPermission || requiredPermission}
          onAction={handleReturnToSafeRoute}
        />
      )
    }

    if (!requiredPermission) {
      return (
        <div
          className="state-container"
          style={{ minHeight: '400px', justifyContent: 'flex-start', alignItems: 'stretch' }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ fontWeight: 'var(--font-weight-semibold)', color: 'var(--color-text-secondary)' }}>
              {t('status.systemReady')} — {routeTitle}
            </span>
          </div>
        </div>
      )
    }

    return (
      <PermissionGate
        permission={requiredPermission}
        overrides={effectiveOverrides}
        fallback={
          <PermissionDeniedState
            permission={requiredPermission}
            onAction={handleReturnToSafeRoute}
          />
        }
      >
        <WorkspaceModuleView
          activeRoute={activeRoute}
          routeTitle={routeTitle}
          systemReadyText={t('status.systemReady')}
        />
      </PermissionGate>
    )
  }

  return (
    <main
      id="main-content"
      className="app-main"
      tabIndex={-1}
      role="main"
      data-testid="main-content"
    >
      <div className="app-content">
        {/* Non-blocking Offline Banner */}
        {!isOnline && <OfflineBanner pendingCount={pendingSyncCount} />}

        {/* Breadcrumb Navigation */}
        <nav aria-label={t('app.breadcrumb')}>
          <ol className="breadcrumbs">
            <li className="breadcrumbs__item">
              <span>{t('app.name')}</span>
              <span className="breadcrumbs__separator" aria-hidden="true">
                /
              </span>
            </li>
            <li className="breadcrumbs__item breadcrumbs__item--active" aria-current="page">
              <span>{routeTitle}</span>
            </li>
          </ol>
        </nav>

        {/* View Header */}
        <div className="view-header">
          <div>
            <h1 className="view-header__title">{routeTitle}</h1>
            <p className="view-header__subtitle">
              {t('app.tagline')}
            </p>
          </div>
        </div>

        {/* Dynamic State Management */}
        {renderDynamicContent()}
      </div>
    </main>
  )
}
