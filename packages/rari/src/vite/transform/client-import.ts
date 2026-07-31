import type { ScannedImport } from '../analysis/directives'

export interface ClientReferenceReplacement {
  /** Binding statements only (no helper import). Empty when there are no runtime bindings. */
  readonly code: string
  /** Helper names required by `code` (`registerClientReference` / `createClientModuleProxy`). */
  readonly helpers: readonly string[]
}

function buildNamespaceClientReferenceBinding(
  bindingName: string,
  resolvedImportPath: string,
): string {
  return `const ${bindingName} = createClientModuleProxy(${JSON.stringify(resolvedImportPath)});`
}

function buildRegisterClientReferenceBinding(
  bindingName: string,
  resolvedImportPath: string,
  exportName: string,
): string {
  return `const ${bindingName} = registerClientReference(
  function() {
    throw new Error("Attempted to call ${bindingName} from the server but it's on the client. It can only be rendered as a Component or passed to props of a Client Component.");
  },
  ${JSON.stringify(resolvedImportPath)},
  ${JSON.stringify(exportName)}
);`
}

export function buildClientReferenceReplacementFromImport(
  imp: ScannedImport,
  resolvedImportPath: string,
): ClientReferenceReplacement {
  const parts: string[] = []
  const helpers = new Set<string>()

  if (imp.namespaceBinding != null) {
    helpers.add('createClientModuleProxy')
    parts.push(buildNamespaceClientReferenceBinding(imp.namespaceBinding, resolvedImportPath))
  }

  if (imp.defaultBinding != null) {
    helpers.add('registerClientReference')
    parts.push(
      buildRegisterClientReferenceBinding(imp.defaultBinding, resolvedImportPath, 'default'),
    )
  }

  for (const spec of imp.named) {
    if (spec.typeOnly) continue
    helpers.add('registerClientReference')
    parts.push(buildRegisterClientReferenceBinding(spec.local, resolvedImportPath, spec.imported))
  }

  if (parts.length === 0) return { code: '', helpers: [] }

  return { code: parts.join('\n'), helpers: [...helpers] }
}
