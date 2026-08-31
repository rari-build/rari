import { normalizeFlightContent } from '@rari/runtime/flight/normalize-flight-content'
import * as React from 'react'
import { describe, expect, it } from 'vite-plus/test'

describe('normalizeFlightContent', () => {
  it('returns non-array content unchanged', () => {
    const element = React.createElement('div', null, 'hello')
    expect(normalizeFlightContent(element)).toBe(element)
    expect(normalizeFlightContent('text')).toBe('text')
  })

  it('unwraps a single-element flight array', () => {
    const element = React.createElement('main', null, 'page')
    expect(normalizeFlightContent([element])).toBe(element)
  })

  it('wraps renderable multi-item flight arrays in a fragment', () => {
    const normalized = normalizeFlightContent([
      React.createElement('div', { key: 'a' }, 'a'),
      React.createElement('div', { key: 'b' }, 'b'),
    ])

    const isFragment = React.isValidElement(normalized) && normalized.type === React.Fragment
    expect(isFragment).toBe(true)
  })

  it('returns mixed arrays unchanged when items are not all renderable', () => {
    const mixed = [{ not: 'a react node' }]
    // @ts-expect-error simulates invalid deserialized flight array
    const result = normalizeFlightContent(mixed)
    expect(result).toBe(mixed)
  })
})
