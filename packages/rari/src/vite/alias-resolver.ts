import path from 'node:path'

export function resolveAlias(
  source: string,
  aliases: Record<string, string>,
  projectRoot: string,
): string | null {
  const sortedAliases = Object.entries(aliases).sort((a, b) => b[0].length - a[0].length)

  for (const [alias, replacement] of sortedAliases) {
    if (source.startsWith(`${alias}/`) || source === alias) {
      const relativePath = source.slice(alias.length)
      const newPath = path.join(replacement, relativePath)
      return path.isAbsolute(newPath) ? newPath : path.resolve(projectRoot, newPath)
    }
  }

  return null
}
