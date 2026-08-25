import React from 'react'
import { useTranslation } from 'react-i18next'
import { useShell } from '../../context/ShellContext'
import { OfflineBanner } from '../common/OfflineBanner'
import { LoadingSkeleton } from '../common/LoadingSkeleton'
import { EmptyState } from '../common/EmptyState'
import { ErrorState } from '../common/ErrorState'
import { PermissionDeniedState } from '../common/PermissionDeniedState'

export const MainContent: React.FC = () => {
  const { t } = useTranslation()
  const {
    activeRoute,
    isOnline,
    pendingSyncCount,
    viewState,
    setViewState,
    errorMessage,
    deniedPermission,
  } = useShell()

  const routeTitleKey = `nav.items.${activeRoute}`
  const routeTitle = t(routeTitleKey)

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
        {viewState === 'loading' && <LoadingSkeleton />}
        {viewState === 'empty' && (
          <EmptyState
            onAction={() => setViewState('idle')}
          />
        )}
        {viewState === 'error' && (
          <ErrorState
            message={errorMessage}
            onRetry={() => setViewState('idle')}
          />
        )}
        {viewState === 'permission-denied' && (
          <PermissionDeniedState
            permission={deniedPermission}
            onAction={() => setViewState('idle')}
          />
        )}
        {viewState === 'idle' && (
          <div
            className="state-container"
            style={{ minHeight: '400px', justifyContent: 'flex-start', alignItems: 'stretch' }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ fontWeight: 'var(--font-weight-semibold)', color: 'var(--color-text-secondary)' }}>
                {t('status.systemReady')} — {routeTitle}
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
        )}
      </div>
    </main>
  )
}
