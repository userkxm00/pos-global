import React from 'react'
import { useTranslation } from 'react-i18next'
import { useShell, NavigationRoute } from '../../context/ShellContext'
import {
  PosIcon,
  ShiftIcon,
  InventoryIcon,
  CustomersIcon,
  ReportsIcon,
  UsersIcon,
  TenantsIcon,
  SettingsIcon,
} from '../common/Icons'

interface NavItemConfig {
  id: NavigationRoute
  labelKey: string
  shortcutKey?: string
  icon: React.ReactNode
}

interface NavSectionConfig {
  titleKey: string
  items: NavItemConfig[]
}

function getCollapsedTitle(isCollapsed: boolean, label: string, shortcut: string | null): string | undefined {
  if (!isCollapsed) {
    return undefined
  }
  if (shortcut) {
    return `${label} (${shortcut})`
  }
  return label
}

export const Sidebar: React.FC = () => {
  const { t } = useTranslation()
  const { activeRoute, setActiveRoute, sidebarCollapsed, toggleSidebar } = useShell()

  const sections: NavSectionConfig[] = [
    {
      titleKey: 'nav.sections.operations',
      items: [
        {
          id: 'pos',
          labelKey: 'nav.items.pos',
          shortcutKey: 'shortcuts.pos',
          icon: <PosIcon />,
        },
        {
          id: 'shifts',
          labelKey: 'nav.items.shifts',
          shortcutKey: 'shortcuts.shifts',
          icon: <ShiftIcon />,
        },
      ],
    },
    {
      titleKey: 'nav.sections.management',
      items: [
        {
          id: 'inventory',
          labelKey: 'nav.items.inventory',
          icon: <InventoryIcon />,
        },
        {
          id: 'customers',
          labelKey: 'nav.items.customers',
          icon: <CustomersIcon />,
        },
        {
          id: 'reports',
          labelKey: 'nav.items.reports',
          icon: <ReportsIcon />,
        },
      ],
    },
    {
      titleKey: 'nav.sections.administration',
      items: [
        {
          id: 'users',
          labelKey: 'nav.items.users',
          icon: <UsersIcon />,
        },
        {
          id: 'tenants',
          labelKey: 'nav.items.tenants',
          icon: <TenantsIcon />,
        },
        {
          id: 'settings',
          labelKey: 'nav.items.settings',
          icon: <SettingsIcon />,
        },
      ],
    },
  ]

  return (
    <aside
      className={`app-sidebar ${sidebarCollapsed ? 'app-sidebar--collapsed' : ''}`}
      data-testid="app-sidebar"
    >
      <nav className="app-sidebar__nav" aria-label={t('app.name')}>
        {sections.map((sec) => (
          <div key={sec.titleKey} className="nav-section">
            {!sidebarCollapsed && (
              <span className="nav-section__title" id={`nav-section-${sec.titleKey}`}>
                {t(sec.titleKey)}
              </span>
            )}
            {sec.items.map((item) => {
              const isActive = activeRoute === item.id
              const itemLabel = t(item.labelKey)
              const shortcut = item.shortcutKey ? t(item.shortcutKey) : null
              const itemTitle = getCollapsedTitle(sidebarCollapsed, itemLabel, shortcut)

              return (
                <button
                  key={item.id}
                  type="button"
                  className={`nav-item ${isActive ? 'nav-item--active' : ''}`}
                  onClick={() => setActiveRoute(item.id)}
                  aria-current={isActive ? 'page' : undefined}
                  title={itemTitle}
                  aria-label={itemLabel}
                >
                  <span className="nav-item__icon">{item.icon}</span>
                  {!sidebarCollapsed && (
                    <>
                      <span className="nav-item__label">{itemLabel}</span>
                      {shortcut && <span className="nav-item__shortcut">{shortcut}</span>}
                    </>
                  )}
                </button>
              )
            })}
          </div>
        ))}
      </nav>

      <div className="app-sidebar__footer">
        <button
          type="button"
          className="sidebar-toggle-btn"
          onClick={toggleSidebar}
          aria-expanded={!sidebarCollapsed}
          aria-label={sidebarCollapsed ? t('nav.expand') : t('nav.collapse')}
          title={sidebarCollapsed ? t('nav.expand') : t('nav.collapse')}
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            {sidebarCollapsed ? (
              <polyline points="13 17 18 12 13 7" />
            ) : (
              <polyline points="11 17 6 12 11 7" />
            )}
          </svg>
          {!sidebarCollapsed && <span>{t('nav.collapse')}</span>}
        </button>
      </div>
    </aside>
  )
}
