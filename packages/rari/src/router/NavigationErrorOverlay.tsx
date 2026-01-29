'use client'

import type { NavigationError } from './navigation-error-handler'

export interface NavigationErrorOverlayProps {
  error: NavigationError
  onRetry?: () => void
  onReload?: () => void
  onDismiss?: () => void
  retryCount?: number
  maxRetries?: number
}

export function NavigationErrorOverlay({
  error,
  onRetry,
  onReload,
  onDismiss,
  retryCount = 0,
  maxRetries = 3,
}: NavigationErrorOverlayProps) {
  const canRetry = error.retryable && retryCount < maxRetries

  const getErrorMessage = (): string => {
    switch (error.type) {
      case 'timeout':
        return 'The page took too long to load. Please try again.'
      case 'network-error':
        return 'Unable to connect to the server. Please check your internet connection.'
      case 'not-found':
        return 'The page you\'re looking for doesn\'t exist.'
      case 'server-error':
        return 'The server encountered an error. Please try again in a moment.'
      case 'parse-error':
        return 'Unable to load the page content. Please try reloading.'
      case 'abort':
        return 'Navigation was cancelled.'
      default:
        return error.message || 'An unexpected error occurred.'
    }
  }

  const getErrorIcon = (): string => {
    switch (error.type) {
      case 'timeout':
        return '⏱️'
      case 'network-error':
        return '📡'
      case 'not-found':
        return '🔍'
      case 'server-error':
        return '⚠️'
      case 'parse-error':
        return '📄'
      case 'abort':
        return '🚫'
      default:
        return '❌'
    }
  }

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: 'rgba(0, 0, 0, 0.5)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 10000,
        padding: '20px',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget && onDismiss)
          onDismiss()
      }}
    >
      <div
        style={{
          background: 'white',
          borderRadius: '8px',
          padding: '24px',
          maxWidth: '500px',
          width: '100%',
          boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
        }}
        onClick={e => e.stopPropagation()}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '12px',
            marginBottom: '16px',
          }}
        >
          <span style={{ fontSize: '32px' }}>{getErrorIcon()}</span>
          <h2
            style={{
              margin: 0,
              fontSize: '20px',
              fontWeight: '600',
              color: '#1f2937',
            }}
          >
            Navigation Error
          </h2>
        </div>

        <p
          style={{
            margin: '0 0 16px 0',
            fontSize: '15px',
            lineHeight: '1.5',
            color: '#4b5563',
          }}
        >
          {getErrorMessage()}
        </p>

        {error.url && (
          <details
            style={{
              marginBottom: '16px',
              fontSize: '13px',
              color: '#6b7280',
            }}
          >
            <summary
              style={{
                cursor: 'pointer',
                userSelect: 'none',
                marginBottom: '8px',
              }}
            >
              Technical details
            </summary>
            <div
              style={{
                padding: '12px',
                background: '#f9fafb',
                borderRadius: '4px',
                fontFamily: 'monospace',
                fontSize: '12px',
                wordBreak: 'break-all',
              }}
            >
              <div style={{ marginBottom: '4px' }}>
                <strong>URL:</strong>
                {' '}
                {error.url}
              </div>
              {error.statusCode && (
                <div style={{ marginBottom: '4px' }}>
                  <strong>Status:</strong>
                  {' '}
                  {error.statusCode}
                </div>
              )}
              <div style={{ marginBottom: '4px' }}>
                <strong>Type:</strong>
                {' '}
                {error.type}
              </div>
              {error.originalError && (
                <div>
                  <strong>Error:</strong>
                  {' '}
                  {error.originalError.message}
                </div>
              )}
            </div>
          </details>
        )}

        {canRetry && retryCount > 0 && (
          <div
            style={{
              marginBottom: '16px',
              padding: '8px 12px',
              background: '#fef3c7',
              border: '1px solid #fbbf24',
              borderRadius: '4px',
              fontSize: '13px',
              color: '#92400e',
            }}
          >
            Retry attempt
            {' '}
            {retryCount}
            {' '}
            of
            {' '}
            {maxRetries}
          </div>
        )}

        <div
          style={{
            display: 'flex',
            gap: '8px',
            flexWrap: 'wrap',
          }}
        >
          {canRetry && onRetry && (
            <button
              onClick={onRetry}
              type="button"
              style={{
                flex: 1,
                minWidth: '120px',
                padding: '10px 16px',
                background: '#3b82f6',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                fontSize: '14px',
                fontWeight: '500',
                cursor: 'pointer',
                transition: 'background 0.2s',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = '#2563eb'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = '#3b82f6'
              }}
            >
              Try Again
            </button>
          )}

          {onReload && (
            <button
              onClick={onReload}
              type="button"
              style={{
                flex: 1,
                minWidth: '120px',
                padding: '10px 16px',
                background: '#6b7280',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                fontSize: '14px',
                fontWeight: '500',
                cursor: 'pointer',
                transition: 'background 0.2s',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = '#4b5563'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = '#6b7280'
              }}
            >
              Reload Page
            </button>
          )}

          {onDismiss && (
            <button
              onClick={onDismiss}
              type="button"
              style={{
                flex: 1,
                minWidth: '120px',
                padding: '10px 16px',
                background: 'transparent',
                color: '#6b7280',
                border: '1px solid #d1d5db',
                borderRadius: '6px',
                fontSize: '14px',
                fontWeight: '500',
                cursor: 'pointer',
                transition: 'all 0.2s',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = '#f9fafb'
                e.currentTarget.style.borderColor = '#9ca3af'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'transparent'
                e.currentTarget.style.borderColor = '#d1d5db'
              }}
            >
              Dismiss
            </button>
          )}
        </div>

        <p
          style={{
            marginTop: '16px',
            marginBottom: 0,
            fontSize: '12px',
            color: '#9ca3af',
            textAlign: 'center',
          }}
        >
          Your current page is still available below this message
        </p>
      </div>
    </div>
  )
}
