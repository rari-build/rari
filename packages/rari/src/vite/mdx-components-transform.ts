import fs from 'node:fs'
import path from 'node:path'
import { BACKSLASH_REGEX } from '../shared/regex-constants'
import { resolveIndexFile, resolveWithExtensions } from '../shared/utils/file-resolver'
import { getProjectRelativePath } from './component-ids'
import { analyzeModuleSource } from './directives'

const DEFAULT_IMPORT_REGEX = /import\s+(\w+)\s+from\s+['"]([^'"]+)['"]/g
const NAMED_IMPORT_REGEX = /import\s+\{([^}]+)\}\s+from\s+['"]([^'"]+)['"]/g
const DEFINE_MDX_CALL_REGEX = /defineMdxComponents\s*\(\s*\{([\s\S]*?)\}\s*\)/

interface TransformDefineMdxComponentsOptions {
  code: string
  id: string
  projectRoot: string
  resolvedAlias: Record<string, string>
}

function parseImportBindings(code: string): Map<string, string> {
  const bindings = new Map<string, string>()

  for (const match of code.matchAll(DEFAULT_IMPORT_REGEX))
    bindings.set(match[1]!, match[2]!)

  for (const match of code.matchAll(NAMED_IMPORT_REGEX)) {
    const source = match[2]!
    for (const part of match[1]!.split(',')) {
      const trimmed = part.trim()
      if (!trimmed)
        continue

      const asParts = trimmed.split(/\s+as\s+/i)
      const localName = (asParts[1] ?? asParts[0])?.trim()
      if (localName)
        bindings.set(localName, source)
    }
  }

  return bindings
}

function parseDefineMdxComponentNames(objectBody: string): Array<{ name: string, binding: string }> {
  const entries: Array<{ name: string, binding: string }> = []

  for (const part of objectBody.split(',')) {
    const trimmed = part.trim()
    if (!trimmed)
      continue

    const colonIndex = trimmed.indexOf(':')
    if (colonIndex === -1) {
      entries.push({ name: trimmed, binding: trimmed })
      continue
    }

    const name = trimmed.slice(0, colonIndex).trim()
    const binding = trimmed.slice(colonIndex + 1).trim()
    if (name && binding)
      entries.push({ name, binding })
  }

  return entries
}

function resolveImportToFilePath(
  importPath: string,
  importerPath: string,
  resolvedAlias: Record<string, string>,
): string {
  let resolvedImportPath = importPath
  for (const [alias, replacement] of Object.entries(resolvedAlias)) {
    if (importPath.startsWith(`${alias}/`)) {
      resolvedImportPath = importPath.replace(alias, replacement)
      break
    }
    else if (importPath === alias) {
      resolvedImportPath = replacement
      break
    }
  }

  const resolvedPath = path.resolve(path.dirname(importerPath), resolvedImportPath)
  const extensions = ['.tsx', '.jsx', '.ts', '.js']
  const withExt = resolveWithExtensions(resolvedPath, extensions)
  if (withExt)
    return withExt

  const indexFile = resolveIndexFile(resolvedPath, extensions)
  if (indexFile)
    return indexFile

  return `${resolvedPath}.tsx`
}

function isClientComponent(filePath: string): boolean {
  try {
    const source = fs.readFileSync(filePath, 'utf-8')
    return analyzeModuleSource(source).topLevelUseClient
  }
  catch {
    return false
  }
}

export function transformDefineMdxComponents(options: TransformDefineMdxComponentsOptions): string | null {
  const { code, id, projectRoot, resolvedAlias } = options

  if (!code.includes('defineMdxComponents'))
    return null

  if (code.includes('__RARI_MDX_RESOLVED__'))
    return null

  const callMatch = code.match(DEFINE_MDX_CALL_REGEX)
  if (!callMatch)
    return null

  const importBindings = parseImportBindings(code)
  const componentEntries = parseDefineMdxComponentNames(callMatch[1]!)
  if (componentEntries.length === 0)
    return null

  const resolvedEntries = componentEntries.map(({ name, binding }) => {
    const importPath = importBindings.get(binding)
    if (!importPath) {
      throw new Error(
        `[rari/mdx] Could not resolve import for MDX component "${name}" (binding "${binding}") in ${id}`,
      )
    }

    const absolutePath = resolveImportToFilePath(importPath, id, resolvedAlias)
    const moduleId = getProjectRelativePath(absolutePath, projectRoot).replace(BACKSLASH_REGEX, '/')
    const client = isClientComponent(absolutePath)

    return `  { name: ${JSON.stringify(name)}, component: ${binding}, id: ${JSON.stringify(moduleId)}, client: ${client} }`
  })

  const replacement = `defineMdxComponents(/* __RARI_MDX_RESOLVED__ */[\n${resolvedEntries.join(',\n')},\n])`

  return code.replace(DEFINE_MDX_CALL_REGEX, replacement)
}
