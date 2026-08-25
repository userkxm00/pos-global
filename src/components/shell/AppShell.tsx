import React, { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useShell } from '../../context/ShellContext'
import { Header } from './Header'
import { Sidebar } from './Sidebar'
import { MainContent } from './MainContent'
import { StatusBar } from './StatusBar'

export const AppShellContent: React.FC = () => {
  const { t } = useTranslation()
  const { setActiveRoute, lockSession, direction } = useShell()

  // Global POS Keyboard Shortcut Listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'F1') {
        e.preventDefault()
        setActiveRoute('pos')
      } else if (e.key === 'F2') {
        e.preventDefault()
        setActiveRoute('shifts')
      } else if (e.key === 'F9') {
        e.preventDefault()
        lockSession()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [setActiveRoute, lockSession])

  return (
    <div className="app-shell" dir={direction} data-testid="app-shell">
      {/* Accessible Skip Link */}
      <a href="#main-content" className="skip-link">
        {t('app.skipToContent')}
      </a>

      {/* App Shell Sections */}
      <Header />
      <Sidebar />
      <MainContent />
      <StatusBar />
    </div>
  )
}

export const AppShell: React.FC = () => {
  return <AppShellContent />
}
