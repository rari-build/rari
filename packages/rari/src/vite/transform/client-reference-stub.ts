const RSC_REFERENCES_IMPORT = 'react-server-dom-rari/server'

/**
 * Emit a `\0client-ref:` stub module that exposes every export from the
 * underlying client file as a registerClientReference binding — not just
 * default. Named imports that fall through to this safety net must resolve.
 */
export function buildClientReferenceStubModule(
  componentId: string,
  exportNames: readonly string[],
): string {
  const names = exportNames.length > 0 ? exportNames : ['default']
  const lines = [
    `import { registerClientReference } from ${JSON.stringify(RSC_REFERENCES_IMPORT)};`,
  ]

  for (const name of names) {
    if (name === 'default') {
      lines.push(
        `export default registerClientReference(null, ${JSON.stringify(componentId)}, "default");`,
      )
    } else {
      lines.push(
        `export const ${name} = registerClientReference(null, ${JSON.stringify(componentId)}, ${JSON.stringify(name)});`,
      )
    }
  }

  return `${lines.join('\n')}\n`
}

/**
 * Collect export names from source for client-ref stubs and server-action
 * proxies. Covers declarations and `export { a as b }` lists.
 */
export function collectExportNames(code: string): string[] {
  const exports = new Set<string>()

  if (/export\s+default\b/.test(code)) exports.add('default')

  for (const m of code.matchAll(
    /export\s+(?:async\s+)?(?:function|const|let|var|class)\s+(\w+)/g,
  )) {
    exports.add(m[1])
  }

  for (const m of code.matchAll(/export\s*\{([^}]+)\}/g)) {
    for (const part of m[1].split(',')) {
      const trimmed = part.trim()
      if (!trimmed || trimmed.startsWith('type ')) continue

      const asParts = trimmed.split(/\s+as\s+/)
      const exportedName = (asParts.at(-1) ?? '').trim()
      if (exportedName === '') continue
      exports.add(exportedName)
    }
  }

  return exports.size > 0 ? [...exports] : ['default']
}
