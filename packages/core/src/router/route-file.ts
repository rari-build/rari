/**
 * rari's route-file grammar.
 *
 * A route directory names its files by *role*; a framework's
 * {@link RouteConventions} say which base name plays which role (`page` or
 * `index`, `layout`, `not-found` or `404`, …). On top of the role name rari
 * recognises one small grammar, identical for every framework, that lets a
 * page choose the layouts that wrap it:
 *
 * | File              | Meaning                                                        |
 * | ----------------- | -------------------------------------------------------------- |
 * | `page.tsx`        | the directory's page, wrapped by the default layout chain      |
 * | `page@wide.tsx`   | the page selects the `wide` layout variant                     |
 * | `page!.tsx`       | the page opts out of every layout                              |
 * | `layout.tsx`      | the directory's default layout                                 |
 * | `layout-wide.tsx` | the `wide` layout variant; only pages that select it use it    |
 * | `layout!.tsx`     | a top layout: the layouts above it are skipped                 |
 *
 * (`page` and `layout` stand for whatever base names the conventions use.)
 *
 * The scanner records the selection on the manifest (`layout` / `skipLayouts`
 * on a page, `name` / `skipParents` on a layout); the host's router resolves
 * the chain per request: walking from the page's directory to the root, only
 * the selected variant counts until it is found, after which every directory
 * contributes its default layout, and a top layout ends the walk. A variant no
 * page selects is not a layout at all, so colocated `layout-*.tsx` helpers in
 * a directory stay plain files.
 */

/** A file the scanner recognised for a role, with the grammar parsed off its name. */
export interface RouteFile {
  /** File name including the extension, e.g. `page@wide.tsx`. */
  readonly fileName: string
  /** Layout variant the file selects (`page@wide`) or provides (`layout-wide`). */
  readonly variant?: string
  /** The `!` modifier: a page skips every layout, a layout skips its ancestors. */
  readonly bang: boolean
}

/** Which side of the grammar a role sits on: pages select variants, layouts provide them. */
export type RouteFileKind = 'page' | 'layout'

const VARIANT_SEPARATOR: Record<RouteFileKind, string> = { page: '@', layout: '-' }

/**
 * Parse `fileName` as the given role. Returns `undefined` when the file does
 * not play that role: wrong base name, unknown extension, or a suffix the
 * grammar does not define (so `pages.tsx` is not a page and `layout-helpers`
 * is only a layout variant named `helpers`, which the scanner drops unless a
 * page selects it).
 */
export function parseRouteFile(
  fileName: string,
  role: string,
  kind: RouteFileKind,
  extensions: readonly string[],
): RouteFile | undefined {
  if (role === '') return undefined
  const extension = extensions.find(candidate => fileName.endsWith(candidate))
  if (extension == null) return undefined
  const stem = fileName.slice(0, -extension.length)
  if (!stem.startsWith(role)) return undefined

  let rest = stem.slice(role.length)
  if (rest === '') return { fileName, bang: false }
  const bang = rest.endsWith('!')
  if (bang) rest = rest.slice(0, -1)
  if (rest === '') return { fileName, bang: true }

  const separator = VARIANT_SEPARATOR[kind]
  if (!rest.startsWith(separator) || rest.length < 2) return undefined
  return { fileName, variant: rest.slice(separator.length), bang }
}

/**
 * Every file in a directory that plays `role`, the plain one first (so the
 * default layout precedes its variants and a lone page is `[0]`), then by name.
 */
export function findRouteFiles(
  files: readonly string[],
  role: string,
  kind: RouteFileKind,
  extensions: readonly string[],
): RouteFile[] {
  const matches: RouteFile[] = []
  for (const file of files) {
    const parsed = parseRouteFile(file, role, kind, extensions)
    if (parsed) matches.push(parsed)
  }
  return matches.sort((a, b) => {
    const aPlain = a.variant == null && !a.bang
    const bPlain = b.variant == null && !b.bang
    if (aPlain !== bPlain) return aPlain ? -1 : 1
    return a.fileName.localeCompare(b.fileName)
  })
}
