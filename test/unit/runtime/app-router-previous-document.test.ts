import { resolvePreviousDocument } from '@rari/runtime/flight/resolve-previous-document'
import * as React from 'react'
import { describe, expect, it } from 'vite-plus/test'
import { fulfilledFlightNode, pendingFlightNode } from '../../helpers/flight-thenable'

describe('resolvePreviousDocument (soft-nav merge baseline)', () => {
  it('accepts a fulfilled Flight thenable wrapping the committed html document', () => {
    const document = React.createElement(
      'html',
      { lang: 'en' },
      React.createElement('body', null, 'home'),
    )

    const resolved = resolvePreviousDocument(fulfilledFlightNode(document))
    expect(resolved?.type).toBe('html')
    expect(React.isValidElement<{ lang?: string }>(resolved) && resolved.props.lang).toBe('en')
  })

  it('rejects a pending Flight thenable', () => {
    expect(resolvePreviousDocument(pendingFlightNode())).toBeNull()
  })
})
