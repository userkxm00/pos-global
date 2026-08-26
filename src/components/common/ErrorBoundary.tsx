// React Error Boundary Component
// F1.18 — Authorization and Error-State UX & UI_SPEC.md

import React, { Component, ErrorInfo } from 'react'
import i18n from '../../i18n'
import { ErrorState } from './ErrorState'

export interface ErrorBoundaryProps {
  children: React.ReactNode
  fallback?: React.ReactNode
  onError?: (error: Error, errorInfo: ErrorInfo) => void
}

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = {
      hasError: false,
      error: null,
    }
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return {
      hasError: true,
      error,
    }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    if (this.props.onError) {
      this.props.onError(error, errorInfo)
    }
  }

  handleRetry = (): void => {
    this.setState({
      hasError: false,
      error: null,
    })
  }

  handleReload = (): void => {
    if (typeof window !== 'undefined') {
      window.location.reload()
    }
  }

  render(): React.ReactNode {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback
      }

      const title = i18n.t('errorBoundary.title')
      const defaultDesc = i18n.t('errorBoundary.description')
      const retryLabel = i18n.t('errorBoundary.tryAgain')
      const reloadLabel = i18n.t('errorBoundary.reloadApp')

      return (
        <ErrorState
          title={title}
          message={this.state.error?.message || defaultDesc}
          errorCode="ERR_RENDER_FAILURE"
          retryLabel={retryLabel}
          reportLabel={reloadLabel}
          onRetry={this.handleRetry}
          onReport={this.handleReload}
        />
      )
    }

    return this.props.children
  }
}
