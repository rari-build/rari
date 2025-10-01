'use client'

import React from 'react'
import { createRoot } from 'react-dom/client'

import Counter from './components/Counter'
import TodoList from './components/TodoList'

if (typeof globalThis.__clientComponents === 'undefined') {
  globalThis.__clientComponents = {}
}
if (typeof globalThis.__clientComponentPaths === 'undefined') {
  globalThis.__clientComponentPaths = {}
}

globalThis.__clientComponents['Counter'] = {
  id: 'Counter',
  path: 'src/components/Counter.tsx',
  type: 'client',
  component: Counter,
  registered: true,
}
globalThis.__clientComponentPaths['src/components/Counter.tsx'] = 'Counter'

globalThis.__clientComponents['TodoList'] = {
  id: 'TodoList',
  path: 'src/components/TodoList.tsx',
  type: 'client',
  component: TodoList,
  registered: true,
}
globalThis.__clientComponentPaths['src/components/TodoList.tsx'] = 'TodoList'

const CLIENT_COMPONENTS: Record<string, any> = {
  'src/components/Counter.tsx': Counter,
  'src/components/TodoList.tsx': TodoList,
}

console.warn('[RSC Client] Entry client loaded!', {
  Counter: globalThis.__clientComponents['Counter'],
  TodoList: globalThis.__clientComponents['TodoList'],
})

export async function renderApp() {
  console.warn('[RSC Client] renderApp() called')

  const rootElement = document.getElementById('root')
  if (!rootElement) {
    console.error('[RSC Client] Root element not found')
    return
  }

  console.warn('[RSC Client] Root element found:', rootElement)

  try {
    const rariServerUrl = 'http://localhost:3000'
    const url = rariServerUrl + window.location.pathname + window.location.search
    console.warn('[RSC Client] Fetching RSC data from:', url)

    const response = await fetch(url, {
      headers: {
        'Accept': 'text/x-component',
      },
    })

    console.warn('[RSC Client] Response status:', response.status)
    console.warn('[RSC Client] Response headers:', Object.fromEntries(response.headers.entries()))

    if (!response.ok) {
      throw new Error(`Failed to fetch RSC data: ${response.status}`)
    }

    const rscWireFormat = await response.text()
    console.warn('[RSC Client] RSC Wire Format received (first 500 chars):', rscWireFormat.substring(0, 500))
    console.warn('[RSC Client] Total length:', rscWireFormat.length)

    const { element, modules } = parseRscWireFormat(rscWireFormat)

    console.warn('[RSC Client] Parsed RSC element:', element)
    console.warn('[RSC Client] Client modules:', modules)

    const root = createRoot(rootElement)
    console.warn('[RSC Client] Rendering element...')
    root.render(element)
    console.warn('[RSC Client] Render complete!')
  } catch (error) {
    console.error('[RSC Client] Error rendering app:', error)
    rootElement.innerHTML = `
      <div style="padding: 20px; background: #fee; border: 2px solid #f00; margin: 20px;">
        <h2>Error Loading App</h2>
        <p>${error instanceof Error ? error.message : String(error)}</p>
        <pre style="background: #f5f5f5; padding: 10px; overflow: auto;">${error instanceof Error ? error.stack : ''}</pre>
      </div>
    `
  }
}

function parseRscWireFormat(wireFormat: string): { element: React.ReactElement; modules: Map<string, any> } {
  const lines = wireFormat.trim().split('\n')
  const modules = new Map<string, any>()
  let rootElement: any = null

  for (const line of lines) {
    if (!line.trim()) continue

    const colonIndex = line.indexOf(':')
    if (colonIndex === -1) continue

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
        }
      }
      else if (content.startsWith('[')) {
        const elementData = JSON.parse(content)
        if (!rootElement && Array.isArray(elementData) && elementData[0] === '$') {
          rootElement = rscToReact(elementData, modules)
        }
      }
    } catch (e) {
      console.error('Failed to parse RSC line:', line, e)
    }
  }

  if (!rootElement) {
    throw new Error('No root element found in RSC wire format')
  }

  return { element: rootElement, modules }
}

function rscToReact(rsc: any, modules: Map<string, any>): any {
  if (!rsc) return null

  if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean') {
    return rsc
  }

  if (Array.isArray(rsc)) {
    if (rsc.length >= 4 && rsc[0] === '$') {
      const [, type, key, props] = rsc

      if (typeof type === 'string' && type.startsWith('$L')) {
        const moduleInfo = modules.get(type)
        if (moduleInfo) {
          const Component = CLIENT_COMPONENTS[moduleInfo.id]
          if (Component) {
            console.log(`[RSC Client] Rendering client component: ${moduleInfo.id}`)
            const childProps = {
              ...props,
              children: props.children ? rscToReact(props.children, modules) : undefined,
            }
            return React.createElement(Component, { key, ...childProps })
          } else {
            console.warn(`Client component not found: ${moduleInfo.id}`)
            return React.createElement('div', {
              key,
              style: {
                border: '2px dashed #f00',
                padding: '10px',
                margin: '10px',
                background: '#fff0f0',
              },
            }, `[Client Component Not Found: ${moduleInfo.name} from ${moduleInfo.id}]`)
          }
        }
      }

      const processedProps = processProps(props, modules)
      return React.createElement(type, key ? { ...processedProps, key } : processedProps)
    }

    return rsc.map((child) => rscToReact(child, modules))
  }

  return rsc
}

function processProps(props: any, modules: Map<string, any>): any {
  if (!props || typeof props !== 'object') return props

  const processed: any = {}

  for (const [key, value] of Object.entries(props)) {
    if (key === 'children') {
      processed[key] = rscToReact(value, modules)
    } else {
      processed[key] = value
    }
  }

  return processed
}

console.warn('[RSC Client] Starting app...')
renderApp().catch((err) => {
  console.error('[RSC Client] Fatal error:', err)
})

