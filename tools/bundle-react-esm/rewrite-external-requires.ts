export function packageStillRequired(code: string, pkg: string): boolean {
  const escaped = pkg.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return new RegExp(`(?:__require|\\b[A-Za-z_$][\\w$]*)\\(([\`"'])${escaped}\\1\\)`).test(code)
}

export function rewriteExternalRequires(
  code: string,
  externals: Readonly<Record<string, string>>,
): string {
  const importLines: string[] = []
  let result = code

  // Unminified Rolldown emits `__require("pkg")`. Minified output uses a short
  // CJS interop helper shaped like `l=(e=>typeof require...` with calls `l(\`pkg\`)`.
  const minifiedHelper = /([A-Za-z_$][\w$]*)=\(e=>typeof require/.exec(result)?.[1]

  for (const [pkg, target] of Object.entries(externals)) {
    const ident = `__ext_${pkg.replace(/\W/g, '_')}`
    const escaped = pkg.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const patterns = [new RegExp(`__require\\((["'])${escaped}\\1\\)`, 'g')]

    if (minifiedHelper != null && minifiedHelper !== '') {
      const helper = minifiedHelper.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      patterns.push(new RegExp(`\\b${helper}\\(([\`"'])${escaped}\\1\\)`, 'g'))
    }

    let matched = false
    for (const pattern of patterns) {
      const next = result.replace(pattern, ident)
      if (next !== result) {
        result = next
        matched = true
      }
    }

    if (matched) importLines.push(`import * as ${ident} from '${target}';`)
  }

  if (importLines.length > 0) result = `${importLines.join('\n')}\n${result}`
  return result
}

export function assertExternalsRewritten(
  entryName: string,
  code: string,
  externals: Readonly<Record<string, string>>,
): void {
  for (const pkg of Object.keys(externals)) {
    if (packageStillRequired(code, pkg)) {
      throw new Error(
        `Failed to rewrite external require for "${pkg}" in ${entryName}. ` +
          `Minified Rolldown output may have changed shape; update rewriteExternalRequires.`,
      )
    }
  }
}
