import React from 'react'
import { useTranslation } from 'react-i18next'
import { useShell } from '../../context/ShellContext'
import { useAuth } from '../../context/AuthContext'
import { SyncIndicator } from '../common/SyncIndicator'
import { PosIcon, LockIcon } from '../common/Icons'
import { supportedLocales, SupportedLocale } from '../../i18n'

export const Header: React.FC = () => {
  const { t } = useTranslation()
  const {
    organization,
    branch,
    session,
    isOnline,
    isSyncing,
    locale,
    setLocale,
    lockSession,
  } = useShell()

  const { lock, activeUser } = useAuth()

  const orgName = organization?.name || 'Default Organization'
  const branchName = branch?.name || 'Main Branch'
  const userName = activeUser?.full_name || activeUser?.username || session?.full_name || 'Terminal Operator'
  const userRole = activeUser?.role || session?.role || 'cashier'

  const handleLock = () => {
    lockSession()
    lock()
  }

  return (
    <header className="app-header" role="banner" data-testid="app-header">
      <div className="app-header__start">
        <a href="#pos" className="app-brand" aria-label={t('app.name')}>
          <div className="app-brand__logo" aria-hidden="true">
            <PosIcon size={20} strokeWidth={2.5} />
          </div>
          <span>{t('app.name')}</span>
        </a>

        {/* Tenant and Branch Context Indicator */}
        <div className="tenant-badge">
          <span className="tenant-badge__org">{orgName}</span>
          <span className="tenant-badge__divider" aria-hidden="true">
            /
          </span>
          <span>{branchName}</span>
        </div>
      </div>

      <div className="app-header__center">
        <SyncIndicator isOnline={isOnline} isSyncing={isSyncing} />
      </div>

      <div className="app-header__end">
        {/* Language Switcher: Semantic HTML5 fieldset and legend */}
        <fieldset className="locale-switcher">
          <legend className="sr-only">{t('languages.select')}</legend>
          {supportedLocales.map((loc) => (
            <button
              key={loc}
              type="button"
              className={`locale-btn ${locale === loc ? 'locale-btn--active' : ''}`}
              onClick={() => setLocale(loc as SupportedLocale)}
              aria-pressed={locale === loc}
            >
              {loc.toUpperCase()}
            </button>
          ))}
        </fieldset>

        {/* User Info & Quick Lock */}
        <div className="tenant-badge">
          <span>{userName}</span>
          <span className="tenant-badge__divider" aria-hidden="true">
            •
          </span>
          <span style={{ textTransform: 'capitalize' }}>{userRole}</span>
        </div>

        <button
          type="button"
          className="btn btn--secondary btn--sm"
          onClick={handleLock}
          title={`${t('header.lockSession')} (${t('shortcuts.lock')})`}
          aria-label={`${t('header.lockSession')} (${t('shortcuts.lock')})`}
          data-testid="header-lock-button"
        >
          <LockIcon />
          <span>{t('header.lockSession')}</span>
        </button>
      </div>
    </header>
  )
}
