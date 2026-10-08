import { collectExportNames } from '../analysis/directives'

export { collectExportNames }

const RSC_REFERENCES_IMPORT = 'react-server-dom-rari/server'

/**
 * Emit a `\0client-ref:` stub module that exposes every export from the
 * underlying client file as a registerClientReference binding not just
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
