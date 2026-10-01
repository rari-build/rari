import { normalizeFlightContent } from '@rari/runtime/flight/normalize-flight-content'
import { createElement, Fragment, isValidElement } from 'react'
import { describe, expect, it } from 'vite-plus/test'

describe('normalizeFlightContent', () => {
  it('returns non-array content unchanged', () => {
    const element = createElement('div', null, 'hello')
    expect(normalizeFlightContent(element)).toBe(element)
    expect(normalizeFlightContent('text')).toBe('text')
  })

  it('unwraps a single-element flight array', () => {
    const element = createElement('main', null, 'page')
    expect(normalizeFlightContent([element])).toBe(element)
  })

  it('wraps renderable multi-item flight arrays in a fragment', () => {
    const normalized = normalizeFlightContent([
      createElement('div', { key: 'a' }, 'a'),
      createElement('div', { key: 'b' }, 'b'),
    ])

    const isFragment = isValidElement(normalized) && normalized.type === Fragment
    expect(isFragment).toBe(true)
  })

  it('returns mixed arrays unchanged when items are not all renderable', () => {
    const mixed = [{ not: 'a react node' }]
    // @ts-expect-error simulates invalid deserialized flight array
    const result = normalizeFlightContent(mixed)
    expect(result).toBe(mixed)
  })
})
