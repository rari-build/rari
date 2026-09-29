import { unwrapFulfilledFlightNode } from '@rari/runtime/flight/merge-refresh'
import * as React from 'react'
import { describe, expect, it } from 'vite-plus/test'
import { fulfilledFlightNode, pendingFlightNode } from '../../helpers/flight-thenable'

function resolvePreviousDocumentForTest(
  previousElement: React.ReactNode | undefined,
): React.ReactElement | null {
  if (previousElement == null) return null
  const isThenable = (
    value: unknown,
  ): value is { then: unknown; status?: string; value?: unknown } =>
    typeof value === 'object' &&
    value != null &&
    typeof (value as { then?: unknown }).then === 'function'

  if (isThenable(previousElement)) {
    const resolved = unwrapFulfilledFlightNode(previousElement)
    if (resolved === previousElement) return null
    return React.isValidElement(resolved) && (resolved.type === 'html' || resolved.type === 'HTML')
      ? resolved
      : null
  }
  return React.isValidElement(previousElement) &&
    (previousElement.type === 'html' || previousElement.type === 'HTML')
    ? previousElement
    : null
}

describe('resolvePreviousDocument (soft-nav merge baseline)', () => {
  it('accepts a fulfilled Flight thenable wrapping the committed html document', () => {
    const document = React.createElement(
      'html',
      { lang: 'en' },
      React.createElement('body', null, 'home'),
    )

    const resolved = resolvePreviousDocumentForTest(fulfilledFlightNode(document))
    expect(resolved?.type).toBe('html')
    expect(React.isValidElement<{ lang?: string }>(resolved) && resolved.props.lang).toBe('en')
  })

  it('rejects a pending Flight thenable', () => {
    expect(resolvePreviousDocumentForTest(pendingFlightNode())).toBeNull()
  })
})
