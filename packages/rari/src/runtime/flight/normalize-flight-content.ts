import type { Thenable } from 'virtual:react-flight-client'
import * as React from 'react'

function isRenderableFlightItem(item: unknown): boolean {
  return (
    React.isValidElement(item) ||
    item == null ||
    typeof item === 'string' ||
    typeof item === 'number' ||
    typeof item === 'boolean'
  )
}

export function normalizeFlightContent(
  content: React.ReactNode | Thenable<React.ReactNode>,
): React.ReactNode | Thenable<React.ReactNode> {
  if (!Array.isArray(content)) return content

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion Array.isArray widens flight payload arrays to any[]
  const items = content as React.ReactNode[]
  if (items.length === 1 && React.isValidElement(items[0])) return items[0]
  if (items.length > 0 && items.every(isRenderableFlightItem)) {
    return React.createElement(React.Fragment, null, ...items)
  }

  // oxlint-disable-next-line typescript/no-unsafe-type-assertion non-renderable flight arrays are returned unchanged
  return content as React.ReactNode | Thenable<React.ReactNode>
}
