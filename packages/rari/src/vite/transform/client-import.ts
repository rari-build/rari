export const NAMESPACE_IMPORT_LINE_REGEX =
  /^\s*import\s+\*\s+as\s+(\w+)\s+from\s+['"]([./@][^'"]+)['"].*$/

export function buildNamespaceClientReferenceReplacement(
  bindingName: string,
  resolvedImportPath: string,
): string {
  return `import { createClientModuleProxy } from "react-server-dom-rari/server";
const ${bindingName} = createClientModuleProxy(${JSON.stringify(resolvedImportPath)});`
}
