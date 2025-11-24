import type { LayoutBoundary, ParsedRSCPayload, RSCRouteMetadata } from './rsc-types'

export interface ParseRSCOptions {
  extractLayoutBoundaries?: boolean
  currentRoute?: string
  layoutChain?: string[]
}

export function parseRscWireFormat(
  wireFormat: string,
  options: ParseRSCOptions = {},
): ParsedRSCPayload {
  try {
    const lines = wireFormat.trim().split('\n')
    const modules = new Map<string, any>()
    let rootElement = null
    const layoutBoundaries: LayoutBoundary[] = []
    let currentLayoutPath: string | null = null
    let currentLayoutStartLine: number | null = null

    for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
      const line = lines[lineIndex]
      const colonIndex = line.indexOf(':')

      if (colonIndex === -1) {
        continue
      }

      const rowId = line.substring(0, colonIndex)
      const content = line.substring(colonIndex + 1)

      try {
        if (content.startsWith('I[')) {
          const importData = JSON.parse(content.substring(1))
          if (Array.isArray(importData) && importData.length >= 3) {
            const [path, chunks, exportName] = importData
            modules.set(`$L${rowId}`, {
              id: path,
              chunks: Array.isArray(chunks) ? chunks : [chunks],
              name: exportName || 'default',
            })

            if (options.extractLayoutBoundaries && path.includes('layout')) {
              if (currentLayoutPath !== null && currentLayoutStartLine !== null) {
                layoutBoundaries.push({
                  layoutPath: currentLayoutPath,
                  startLine: currentLayoutStartLine,
                  endLine: lineIndex - 1,
                  props: {},
                })
              }

              currentLayoutPath = path
              currentLayoutStartLine = lineIndex
            }
          }
        }
        else if (content.startsWith('[')) {
          const elementData = JSON.parse(content)

          if (
            options.extractLayoutBoundaries
            && Array.isArray(elementData)
            && elementData.length >= 4
            && typeof elementData[1] === 'string'
            && elementData[1].startsWith('$L')
          ) {
            const moduleRef = elementData[1]
            const moduleInfo = modules.get(moduleRef)

            if (moduleInfo && moduleInfo.id.includes('layout')) {
              const props = elementData[3] || {}

              if (currentLayoutPath && currentLayoutStartLine !== null) {
                const existingBoundary = layoutBoundaries.find(
                  b => b.layoutPath === currentLayoutPath && b.startLine === currentLayoutStartLine,
                )

                if (existingBoundary) {
                  existingBoundary.props = props
                }
              }
            }
          }

          if (!rootElement && Array.isArray(elementData) && elementData[0] === '$') {
            rootElement = elementData
          }
        }
      }
      catch (parseError) {
        console.error('[RSC Parser] Failed to parse line:', line, parseError)
      }
    }

    if (
      options.extractLayoutBoundaries
      && currentLayoutPath !== null
      && currentLayoutStartLine !== null
    ) {
      layoutBoundaries.push({
        layoutPath: currentLayoutPath,
        startLine: currentLayoutStartLine,
        endLine: lines.length - 1,
        props: {},
      })
    }

    let routeMetadata: RSCRouteMetadata | undefined
    if (options.extractLayoutBoundaries && options.currentRoute) {
      routeMetadata = {
        route: options.currentRoute,
        layoutChain: options.layoutChain || [],
        timestamp: Date.now(),
      }
    }

    return {
      element: rootElement,
      modules,
      wireFormat,
      layoutBoundaries: options.extractLayoutBoundaries ? layoutBoundaries : undefined,
      routeMetadata,
    }
  }
  catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    throw new Error(`Failed to parse RSC wire format: ${errorMessage}`)
  }
}

export function validateRSCPayload(payload: ParsedRSCPayload): boolean {
  if (!payload) {
    throw new Error('RSC payload is null or undefined')
  }

  if (!payload.wireFormat || typeof payload.wireFormat !== 'string') {
    throw new TypeError('RSC payload missing or invalid wireFormat')
  }

  if (!payload.modules || !(payload.modules instanceof Map)) {
    throw new Error('RSC payload missing or invalid modules map')
  }

  if (payload.layoutBoundaries) {
    if (!Array.isArray(payload.layoutBoundaries)) {
      throw new TypeError('RSC payload layoutBoundaries must be an array')
    }

    for (const boundary of payload.layoutBoundaries) {
      if (!boundary.layoutPath || typeof boundary.layoutPath !== 'string') {
        throw new Error('Layout boundary missing or invalid layoutPath')
      }

      if (typeof boundary.startLine !== 'number' || boundary.startLine < 0) {
        throw new Error('Layout boundary missing or invalid startLine')
      }

      if (typeof boundary.endLine !== 'number' || boundary.endLine < boundary.startLine) {
        throw new Error('Layout boundary missing or invalid endLine')
      }
    }
  }

  if (payload.routeMetadata) {
    if (!payload.routeMetadata.route || typeof payload.routeMetadata.route !== 'string') {
      throw new Error('Route metadata missing or invalid route')
    }

    if (!Array.isArray(payload.routeMetadata.layoutChain)) {
      throw new TypeError('Route metadata missing or invalid layoutChain')
    }

    if (typeof payload.routeMetadata.timestamp !== 'number' || payload.routeMetadata.timestamp <= 0) {
      throw new Error('Route metadata missing or invalid timestamp')
    }
  }

  return true
}

export function extractLayoutBoundaries(
  wireFormat: string,
  layoutChain: string[] = [],
): LayoutBoundary[] {
  const parsed = parseRscWireFormat(wireFormat, {
    extractLayoutBoundaries: true,
    layoutChain,
  })

  return parsed.layoutBoundaries || []
}
