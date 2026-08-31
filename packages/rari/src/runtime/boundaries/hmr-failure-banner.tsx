'use client'

import * as React from 'react'

export type HmrFailureType = 'fetch' | 'parse' | 'stale' | 'network'

export interface HmrFailure {
  readonly timestamp: number
  readonly error: Error
  readonly type: HmrFailureType
  readonly details: string
  readonly filePath?: string
  readonly consecutiveFailures: number
}

export interface HmrFailureBannerProps {
  readonly failure: HmrFailure
  readonly maxRetries: number
  readonly onRefresh: () => void
  readonly onDismiss: () => void
}

const OVERLAY_STYLE: React.CSSProperties = {
  position: 'fixed',
  top: '50%',
  left: '50%',
  transform: 'translate(-50%, -50%)',
  padding: '24px',
  background: 'rgba(220, 38, 38, 0.95)',
  color: 'white',
  borderRadius: '8px',
  fontSize: '14px',
  zIndex: 10000,
  maxWidth: '500px',
  boxShadow: '0 4px 6px rgba(0, 0, 0, 0.3)',
}

const PRIMARY_BUTTON_STYLE: React.CSSProperties = {
  padding: '8px 16px',
  background: 'white',
  color: '#dc2626',
  border: 'none',
  borderRadius: '4px',
  cursor: 'pointer',
  fontWeight: 'bold',
  fontSize: '14px',
}

const SECONDARY_BUTTON_STYLE: React.CSSProperties = {
  padding: '8px 16px',
  background: 'rgba(255, 255, 255, 0.2)',
  color: 'white',
  border: '1px solid rgba(255, 255, 255, 0.3)',
  borderRadius: '4px',
  cursor: 'pointer',
  fontSize: '14px',
}

const FAILURE_MESSAGES: Record<HmrFailureType, string> = {
  fetch: 'Failed to fetch updated content from server.',
  parse: 'Failed to parse server response.',
  stale: 'Server returned stale content.',
  network: 'Network error occurred.',
}

function failureMessage(type: HmrFailureType): string {
  return FAILURE_MESSAGES[type]
}

export function HmrFailureBanner({
  failure,
  maxRetries,
  onRefresh,
  onDismiss,
}: HmrFailureBannerProps): React.ReactNode {
  return (
    <div style={OVERLAY_STYLE}>
      <div style={{ marginBottom: '16px', fontWeight: 'bold', fontSize: '16px' }}>
        ⚠️ HMR Update Failed
      </div>
      <div style={{ marginBottom: '12px', opacity: 0.9 }}>{failureMessage(failure.type)}</div>
      <div
        style={{
          marginBottom: '16px',
          fontSize: '12px',
          opacity: 0.8,
          fontFamily: 'monospace',
        }}
      >
        {failure.details}
      </div>
      <div style={{ marginBottom: '12px', fontSize: '12px', opacity: 0.7 }}>
        Consecutive failures: {failure.consecutiveFailures} / {maxRetries}
      </div>
      <div style={{ display: 'flex', gap: '8px' }}>
        <button onClick={onRefresh} type="button" style={PRIMARY_BUTTON_STYLE}>
          Refresh Page
        </button>
        <button onClick={onDismiss} type="button" style={SECONDARY_BUTTON_STYLE}>
          Dismiss
        </button>
      </div>
      <div style={{ marginTop: '12px', fontSize: '11px', opacity: 0.6 }}>
        Check the console for detailed error logs.
      </div>
    </div>
  )
}
