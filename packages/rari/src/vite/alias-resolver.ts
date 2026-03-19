import path from 'node:path'

export function resolveAlias(
  source: string,
  aliases: Record<string, string>,
  projectRoot: string,
): string | null {
  if (typeof source !== 'string')
    throw new TypeError(`Expected source to be a string, but received ${typeof source}`)

  if (aliases == null)
    throw new TypeError(`Expected aliases to be an object, but received ${aliases}`)
  if (typeof aliases !== 'object' || Array.isArray(aliases))
    throw new TypeError(`Expected aliases to be a plain object, but received ${Array.isArray(aliases) ? 'array' : typeof aliases}`)

  if (typeof projectRoot !== 'string')
    throw new TypeError(`Expected projectRoot to be a string, but received ${typeof projectRoot}`)

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
