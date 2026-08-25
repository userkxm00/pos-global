import React from 'react'
import { useTranslation } from 'react-i18next'
import { useShell } from '../../context/ShellContext'

export const StatusBar: React.FC = () => {
  const { t } = useTranslation()
  const { register, isOnline } = useShell()

  const registerName = register?.name || 'REG-01'

  return (
    <footer className="app-status-bar" role="contentinfo" data-testid="app-status-bar">
      <div className="app-status-bar__item">
        <span>{t('status.currentRegister', { name: registerName })}</span>
        <span style={{ color: 'var(--color-border-strong)' }}>•</span>
        <span>{isOnline ? t('status.online') : t('status.offline')}</span>
      </div>

      <div className="app-status-bar__item" style={{ gap: 'var(--space-4)' }}>
        <span>
          <kbd style={{ padding: '2px 4px', background: 'var(--color-bg-surface-sunken)', borderRadius: '3px', border: '1px solid var(--color-border-subtle)' }}>
            {t('shortcuts.pos')}
          </kbd>{' '}
          {t('nav.items.pos')}
        </span>
        <span>
          <kbd style={{ padding: '2px 4px', background: 'var(--color-bg-surface-sunken)', borderRadius: '3px', border: '1px solid var(--color-border-subtle)' }}>
            {t('shortcuts.shifts')}
          </kbd>{' '}
          {t('nav.items.shifts')}
        </span>
        <span>
          <kbd style={{ padding: '2px 4px', background: 'var(--color-bg-surface-sunken)', borderRadius: '3px', border: '1px solid var(--color-border-subtle)' }}>
            {t('shortcuts.lock')}
          </kbd>{' '}
          {t('header.lockSession')}
        </span>
      </div>
    </footer>
  )
}
