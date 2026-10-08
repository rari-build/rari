import type { ReactNode } from 'react'
import type { FlightContent } from './react-helpers'
import { createElement, Fragment, isValidElement } from 'react'

function isRenderableFlightItem(item: unknown): boolean {
  return (
    isValidElement(item) ||
    item == null ||
    typeof item === 'string' ||
    typeof item === 'number' ||
    typeof item === 'boolean'
  )
}

export function normalizeFlightContent(content: FlightContent): FlightContent {
  if (!Array.isArray(content)) return content

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion Array.isArray widens flight payload arrays to any[]
  const items = content as ReactNode[]
  if (items.length === 1 && isValidElement(items[0])) return items[0]
  if (items.length > 0 && items.every(isRenderableFlightItem)) {
    return createElement(Fragment, null, ...items)
  }

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion non-renderable flight arrays are returned unchanged
  return content as FlightContent
}
