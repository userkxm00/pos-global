import React from 'react'
import { ShellProvider } from './context/ShellContext'
import { AppShell } from './components/shell/AppShell'
import './i18n'
import './styles/global.css'

export default function App() {
  return (
    <ShellProvider>
      <AppShell />
    </ShellProvider>
  )
}
