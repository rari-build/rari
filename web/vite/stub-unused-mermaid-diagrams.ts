import type { Plugin } from 'vite-plus'
import path from 'node:path'

const KEEP_FILE_PREFIXES = [
  'chunk-',
  'flowDiagram-',
  'sequenceDiagram-',
  'dagre-',
  'elk-',
  'sizeCapture-',
] as const

function isMermaidCoreChunk(id: string): boolean {
  return id.replaceAll('\\', '/').includes('/mermaid/dist/chunks/mermaid.core/')
}

function isMermaidCoreEntry(id: string): boolean {
  const normalized = id.replaceAll('\\', '/')
  return normalized.endsWith('/mermaid/dist/mermaid.core.mjs')
}

function shouldKeepMermaidChunk(id: string): boolean {
  const base = path.basename(id).replace(/\.map$/, '')
  return KEEP_FILE_PREFIXES.some(prefix => base.startsWith(prefix))
}

function stubModule(reason: string): string {
  return `throw new Error(${JSON.stringify(reason)})
`
}

function trimRegisteredDiagrams(code: string): {
  readonly code: string
  readonly missing: readonly string[]
} {
  const missing: string[] = []

  let next = code.replace(/if\s*\(true\)\s*\{\s*registerLazyLoadedDiagrams\([^)]+\);\s*\}/, '')
  if (next === code) missing.push('architecture registerLazyLoadedDiagrams(if true) block')

  const afterMain = next.replace(
    /registerLazyLoadedDiagrams\(\s*afDetector_default,[\s\S]*?usecase\s*\);/,
    'registerLazyLoadedDiagrams(sequenceDetector_default, flowDetector_v2_default);',
  )
  if (afterMain === next) missing.push('main registerLazyLoadedDiagrams(afDetector…usecase) call')
  next = afterMain

  let importRewrites = 0
  next = next.replace(
    /await import\("(\.\/chunks\/mermaid\.core\/)([^"]+)"\)/g,
    (full, _prefix: string, file: string) => {
      if (KEEP_FILE_PREFIXES.some(keep => file.startsWith(keep))) return full
      importRewrites += 1
      return `Promise.reject(new Error(${JSON.stringify(`Mermaid diagram "${file}" is not included in this build`)}))`
    },
  )
  if (importRewrites === 0) missing.push('unused await import("./chunks/mermaid.core/…") rewrites')

  return { code: next, missing }
}

export function stubUnusedMermaidDiagrams(): Plugin {
  return {
    name: 'stub-unused-mermaid-diagrams',
    enforce: 'pre',
    resolveId(id) {
      if (id === 'katex' || id.startsWith('katex/')) return '\0stub-katex'
      return null
    },
    load(id) {
      if (id === '\0stub-katex') {
        return `export default {
  renderToString() {
    throw new Error('KaTeX is not bundled; remove $$ math from Mermaid labels or re-enable katex.')
  },
}
`
      }

      if (!isMermaidCoreChunk(id) || id.endsWith('.map')) return null
      if (shouldKeepMermaidChunk(id)) return null

      return stubModule(
        `Mermaid diagram "${path.basename(id)}" is not included in this build. Add its prefix to KEEP_FILE_PREFIXES in web/vite/stub-unused-mermaid-diagrams.ts if needed.`,
      )
    },
    transform(code, id) {
      if (!isMermaidCoreEntry(id)) return null
      const { code: next, missing } = trimRegisteredDiagrams(code)
      if (missing.length > 0) {
        this.warn(
          `[stub-unused-mermaid-diagrams] mermaid.core.mjs pattern drift (${missing.join('; ')}); update the transform`,
        )
      }
      if (next === code) return null
      return { code: next, map: null }
    },
  }
}
