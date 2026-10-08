// oxlint-disable typescript/prefer-readonly-parameter-types
import type { AppIconEntry } from './app-icons'
import type {
  ApiRouteEntry,
  AppRouteEntry,
  AppRouteManifest,
  ErrorEntry,
  LayoutEntry,
  LoadingEntry,
  NotFoundEntry,
  OgImageEntry,
  RouteSegment,
  TemplateEntry,
} from './types'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import { PATH_SEPARATOR_REGEX } from '../regex-constants'
import { toPosixPath } from '../utils/path'
import { discoverAppIconsInDir } from './app-icons'
import { findRouteFiles } from './route-file'

/**
 * Base names of the special files a framework's router recognises inside the
 * app directory. The scanner only cares about *which* file plays which role;
 * what each role means at render time is the framework adapter's business.
 * On top of the base names every framework gets rari's route-file grammar
 * (`page@name`, `page!`, `layout-name`, `layout!`; see `route-file.ts`).
 */
export interface RouteConventions {
  /** Page component for the directory's route (`page` in React, `index` in Qwik). */
  readonly page: string
  readonly layout: string
  readonly loading: string
  readonly error: string
  readonly notFound: string
  readonly template: string
  /** API route handler module. */
  readonly route: string
  readonly ogImage: string
}

/** Next-style conventions: rari's default and the React adapter's. */
export const DEFAULT_ROUTE_CONVENTIONS: RouteConventions = {
  page: 'page',
  layout: 'layout',
  loading: 'loading',
  error: 'error',
  notFound: 'not-found',
  template: 'template',
  route: 'route',
  ogImage: 'opengraph-image',
}

export interface AppRouteGeneratorOptions {
  readonly appDir: string
  readonly extensions?: readonly string[]
  readonly verbose?: boolean
  /** Override any of the special-file names; unspecified ones keep the defaults. */
  readonly conventions?: Partial<RouteConventions>
}

const SEGMENT_PATTERNS = {
  DYNAMIC: /^\[([^\]]+)\]$/,
  CATCH_ALL: /^\[\.\.\.([^\]]+)\]$/,
  OPTIONAL_CATCH_ALL: /^\[\[\.\.\.([^\]]+)\]\]$/,
} as const

const ROUTE_SEGMENT_MATCHERS = [
  {
    pattern: SEGMENT_PATTERNS.OPTIONAL_CATCH_ALL,
    type: 'optional-catch-all' as const,
    format: (param: string) => `[[...${param}]]`,
  },
  {
    pattern: SEGMENT_PATTERNS.CATCH_ALL,
    type: 'catch-all' as const,
    format: (param: string) => `[...${param}]`,
  },
  {
    pattern: SEGMENT_PATTERNS.DYNAMIC,
    type: 'dynamic' as const,
    format: (param: string) => `[${param}]`,
  },
] as const

const GROUP_SEGMENT = /^\([^/]+\)$/

export function isGroupSegment(name: string) {
  return GROUP_SEGMENT.test(name)
}

function isInGroup(filePath: string) {
  if (!filePath) {
    return false
  }

  return toPosixPath(filePath).split('/').filter(Boolean).some(isGroupSegment)
}

function matchRouteSegment(segment: string) {
  for (const matcher of ROUTE_SEGMENT_MATCHERS) {
    const match = segment.match(matcher.pattern)
    if (match) {
      return {
        type: matcher.type,
        param: match[1],
        format: matcher.format,
      }
    }
  }

  return undefined
}

function formatRouteSegment(segment: string) {
  const match = matchRouteSegment(segment)

  return match ? match.format(match.param) : segment
}

function parseRouteSegment(segment: string): RouteSegment {
  const match = matchRouteSegment(segment)
  if (match) {
    return {
      type: match.type,
      value: segment,
      param: match.param,
    }
  }

  return {
    type: 'static',
    value: segment,
  }
}

const SIZE_EXPORT_REGEX =
  /export\s+const\s+size\s*=\s*\{\s*width\s*:\s*(\d+)\s*,\s*height\s*:\s*(\d+)\s*[,}]/
const CONTENT_TYPE_EXPORT_REGEX = /export\s+const\s+contentType\s*=\s*['"]([^'"]+)['"]/

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'] as const

class AppRouteGenerator {
  private readonly appDir: string
  private readonly extensions: string[]
  private readonly verbose: boolean
  private readonly conventions: RouteConventions

  constructor(options: AppRouteGeneratorOptions) {
    this.appDir = path.resolve(options.appDir)
    this.extensions = [...(options.extensions ?? ['.tsx', '.jsx', '.ts', '.js'])]
    this.verbose = options.verbose ?? false
    this.conventions = { ...DEFAULT_ROUTE_CONVENTIONS, ...options.conventions }
  }

  async generateManifest(): Promise<AppRouteManifest> {
    if (this.verbose) console.warn(`[rari] Router: Scanning app directory: ${this.appDir}`)

    const routes: AppRouteEntry[] = []
    const layouts: LayoutEntry[] = []
    const loading: LoadingEntry[] = []
    const errors: ErrorEntry[] = []
    const notFound: NotFoundEntry[] = []
    const templates: TemplateEntry[] = []
    const apiRoutes: ApiRouteEntry[] = []
    const ogImages: OgImageEntry[] = []
    const appIcons: AppIconEntry[] = []

    await this.scanDirectory(
      '',
      routes,
      layouts,
      loading,
      errors,
      notFound,
      templates,
      apiRoutes,
      ogImages,
      appIcons,
    )

    // A layout variant is only a layout when some page selects it; otherwise a
    // colocated `layout-*.tsx` helper stays a plain file.
    const selectedVariants = new Set(
      [...routes, ...notFound].flatMap(page => (page.layout == null ? [] : [page.layout])),
    )
    for (let i = layouts.length - 1; i >= 0; i--) {
      const { name } = layouts[i]
      if (name != null && !selectedVariants.has(name)) {
        if (this.verbose) {
          console.warn(
            `[rari] Router: ignoring layout variant '${layouts[i].filePath}' (no page selects '@${name}')`,
          )
        }
        layouts.splice(i, 1)
      }
    }

    for (const entries of [layouts, loading, errors, notFound, templates, ogImages]) {
      this.finalizeGroupEntries(routes, entries)
    }

    this.assertNoDuplicateRoutes(routes)
    this.assertNoDuplicateRoutes(apiRoutes)

    if (this.verbose) {
      console.warn(`[rari] Router: Found ${routes.length} routes`)
      console.warn(`[rari] Router: Found ${layouts.length} layouts`)
      console.warn(`[rari] Router: Found ${loading.length} loading components`)
      console.warn(`[rari] Router: Found ${errors.length} error boundaries`)
      console.warn(`[rari] Router: Found ${templates.length} templates`)
      console.warn(`[rari] Router: Found ${apiRoutes.length} API routes`)
      console.warn(`[rari] Router: Found ${ogImages.length} OG images`)
      console.warn(`[rari] Router: Found ${appIcons.length} app icons`)
    }

    return {
      routes: this.sortRoutes(routes),
      layouts: this.sortLayouts(layouts),
      loading,
      errors,
      notFound,
      templates: this.sortTemplates(templates),
      apiRoutes: this.sortApiRoutes(apiRoutes),
      ogImages,
      appIcons,
      generated: new Date().toISOString(),
    }
  }

  private finalizeGroupEntries(
    pages: AppRouteEntry[],
    entries: Array<{ path: string; filePath: string; additionalPaths?: string[] }>,
  ): void {
    for (let i = entries.length - 1; i >= 0; i--) {
      const entry = entries[i]
      const fileDir = toPosixPath(path.dirname(entry.filePath))
      if (!isInGroup(fileDir)) {
        continue
      }

      const pagesInSubtree = pages
        .filter(p => {
          const pDir = toPosixPath(path.dirname(p.filePath))

          return pDir === fileDir || pDir.startsWith(`${fileDir}/`)
        })
        .map(p => p.path)

      if (pagesInSubtree.length === 0) {
        entries.splice(i, 1)
        continue
      }

      const uniqueSorted = Array.from(new Set(pagesInSubtree)).sort()
      entry.path = uniqueSorted[0]

      if (uniqueSorted.length > 1) {
        entry.additionalPaths = uniqueSorted.slice(1)
      }
    }
  }

  private assertNoDuplicateRoutes(routes: Array<{ path: string; filePath: string }>): void {
    const seen = new Map<string, string>()
    for (const route of routes) {
      const existing = seen.get(route.path)
      if (existing != null && existing !== '') {
        throw new Error(
          `[rari] Route conflict: path '${route.path}' is defined by both '${existing}' and '${route.filePath}'.`,
        )
      } else {
        seen.set(route.path, route.filePath)
      }
    }
  }

  private async scanDirectory(
    relativePath: string,
    routes: AppRouteEntry[],
    layouts: LayoutEntry[],
    loading: LoadingEntry[],
    errors: ErrorEntry[],
    notFound: NotFoundEntry[],
    templates: TemplateEntry[],
    apiRoutes: ApiRouteEntry[],
    ogImages: OgImageEntry[],
    appIcons: AppIconEntry[],
  ): Promise<void> {
    const fullPath = path.join(this.appDir, relativePath)

    let entries: string[]
    try {
      entries = await fs.readdir(fullPath)
    } catch {
      return
    }

    const files: string[] = []
    const dirs: string[] = []

    for (const entry of entries) {
      const entryPath = path.join(fullPath, entry)
      const stat = await fs.stat(entryPath)

      if (stat.isDirectory()) {
        if (this.shouldScanDirectory(entry)) dirs.push(entry)
      } /* v8 ignore next 3 - edge case: symlinks or special files */ else if (stat.isFile()) {
        files.push(entry)
      }
    }

    await this.processSpecialFiles(
      relativePath,
      files,
      routes,
      layouts,
      loading,
      errors,
      notFound,
      templates,
      apiRoutes,
      ogImages,
      appIcons,
    )

    for (const dir of dirs) {
      const subPath = relativePath ? path.join(relativePath, dir) : dir
      await this.scanDirectory(
        subPath,
        routes,
        layouts,
        loading,
        errors,
        notFound,
        templates,
        apiRoutes,
        ogImages,
        appIcons,
      )
    }
  }

  private async processSpecialFiles(
    relativePath: string,
    files: string[],
    routes: AppRouteEntry[],
    layouts: LayoutEntry[],
    loading: LoadingEntry[],
    errors: ErrorEntry[],
    notFound: NotFoundEntry[],
    templates: TemplateEntry[],
    apiRoutes: ApiRouteEntry[],
    ogImages: OgImageEntry[],
    appIcons: AppIconEntry[],
  ): Promise<void> {
    const routePath = this.pathToRoute(relativePath)

    this.pushPageRoute(relativePath, files, routePath, routes)
    this.pushLayoutEntry(relativePath, files, routePath, layouts)
    this.pushNamedSpecial(relativePath, files, routePath, this.conventions.loading, loading)
    this.pushNamedSpecial(relativePath, files, routePath, this.conventions.error, errors)
    this.pushNotFoundEntry(relativePath, files, routePath, notFound)
    this.pushTemplateEntry(relativePath, files, routePath, templates)
    await this.pushOgImageEntry(relativePath, files, routePath, ogImages)

    const discoveredIcons = await discoverAppIconsInDir({
      appDir: this.appDir,
      relativeDir: relativePath,
      routePath,
      files,
    })
    appIcons.push(...discoveredIcons)

    const routeFile = this.findFile(files, this.conventions.route)
    if (routeFile != null && routeFile !== '') {
      const apiRoute = await this.processApiRouteFile(relativePath, routeFile)
      apiRoutes.push(apiRoute)
    }
  }

  private pushPageRoute(
    relativePath: string,
    files: string[],
    routePath: string,
    routes: AppRouteEntry[],
  ): void {
    const pageFiles = findRouteFiles(files, this.conventions.page, 'page', this.extensions)
    const pageFile = pageFiles.at(0)
    if (pageFile == null) return
    if (pageFiles.length > 1) {
      throw new Error(
        `[rari] Route conflict: '${routePath}' has more than one page file in '${relativePath || '.'}': ${pageFiles.map(file => `'${file.fileName}'`).join(', ')}.`,
      )
    }
    const segments = this.parseRouteSegments(relativePath)
    const params = this.extractParams(segments)
    routes.push({
      path: routePath,
      filePath: toPosixPath(path.join(relativePath, pageFile.fileName)),
      segments,
      params,
      isDynamic: params.length > 0,
      ...(pageFile.variant == null ? {} : { layout: pageFile.variant }),
      ...(pageFile.bang ? { skipLayouts: true } : {}),
    })
  }

  private pushLayoutEntry(
    relativePath: string,
    files: string[],
    routePath: string,
    layouts: LayoutEntry[],
  ): void {
    const parentPath = this.getParentPath(relativePath)
    for (const layoutFile of findRouteFiles(
      files,
      this.conventions.layout,
      'layout',
      this.extensions,
    )) {
      layouts.push({
        path: routePath,
        filePath: toPosixPath(path.join(relativePath, layoutFile.fileName)),
        parentPath: parentPath !== null ? this.pathToRoute(parentPath) : undefined,
        ...(layoutFile.variant == null ? {} : { name: layoutFile.variant }),
        ...(layoutFile.bang ? { skipParents: true } : {}),
      })
    }
  }

  private pushNamedSpecial(
    relativePath: string,
    files: string[],
    routePath: string,
    baseName: string,
    entries: Array<{ path: string; filePath: string }>,
  ): void {
    const file = this.findFile(files, baseName)
    if (file == null || file === '') return
    entries.push({
      path: routePath,
      filePath: toPosixPath(path.join(relativePath, file)),
    })
  }

  /** The not-found page is a page: it can select a layout variant or skip layouts. */
  private pushNotFoundEntry(
    relativePath: string,
    files: string[],
    routePath: string,
    notFound: NotFoundEntry[],
  ): void {
    const file = findRouteFiles(files, this.conventions.notFound, 'page', this.extensions).at(0)
    if (file == null) return
    notFound.push({
      path: routePath,
      filePath: toPosixPath(path.join(relativePath, file.fileName)),
      ...(file.variant == null ? {} : { layout: file.variant }),
      ...(file.bang ? { skipLayouts: true } : {}),
    })
  }

  private pushTemplateEntry(
    relativePath: string,
    files: string[],
    routePath: string,
    templates: TemplateEntry[],
  ): void {
    const templateFile = this.findFile(files, this.conventions.template)
    if (templateFile == null || templateFile === '') return
    const parentPath = this.getParentPath(relativePath)
    templates.push({
      path: routePath,
      filePath: toPosixPath(path.join(relativePath, templateFile)),
      parentPath: parentPath !== null ? this.pathToRoute(parentPath) : undefined,
    })
  }

  private async pushOgImageEntry(
    relativePath: string,
    files: string[],
    routePath: string,
    ogImages: OgImageEntry[],
  ): Promise<void> {
    const ogImageFile = this.findFile(files, this.conventions.ogImage)
    if (ogImageFile == null || ogImageFile === '') return

    const filePath = toPosixPath(path.join(relativePath, ogImageFile))
    const fullFilePath = path.join(this.appDir, filePath)

    let width: number | undefined
    let height: number | undefined
    let contentType: string | undefined

    try {
      const content = await fs.readFile(fullFilePath, 'utf-8')

      const sizeMatch = SIZE_EXPORT_REGEX.exec(content)
      if (sizeMatch) {
        width = Number.parseInt(sizeMatch[1], 10)
        height = Number.parseInt(sizeMatch[2], 10)
      }

      const contentTypeMatch = CONTENT_TYPE_EXPORT_REGEX.exec(content)
      if (contentTypeMatch) contentType = contentTypeMatch[1]
    } catch {}

    ogImages.push({
      path: routePath,
      filePath,
      width,
      height,
      contentType,
    })
  }

  /** Exact role match for roles outside the layout grammar (templates, API routes, OG images, …). */
  private findFile(files: string[], baseName: string): string | undefined {
    // An empty convention name disables that role for the framework.
    if (baseName === '') return undefined
    for (const ext of this.extensions) {
      const fileName = `${baseName}${ext}`
      if (files.includes(fileName)) return fileName
    }
    return undefined
  }

  private pathToRoute(filePath: string): string {
    if (!filePath) return '/'

    const normalized = toPosixPath(filePath)

    const segments = normalized.split('/').filter(Boolean)
    const routeSegments = segments
      .filter(segment => !isGroupSegment(segment))
      .map(formatRouteSegment)

    return `/${routeSegments.join('/')}`
  }

  private parseRouteSegments(filePath: string): RouteSegment[] {
    if (!filePath) return []

    const segments = filePath.split(PATH_SEPARATOR_REGEX).filter(Boolean)
    return segments.filter(segment => !isGroupSegment(segment)).map(parseRouteSegment)
  }

  private extractParams(segments: RouteSegment[]): string[] {
    return segments.filter(seg => seg.param !== undefined).map(seg => seg.param!)
  }

  private getParentPath(filePath: string): string | null {
    if (!filePath) return null

    const parts = filePath.split(PATH_SEPARATOR_REGEX).filter(Boolean)
    /* v8 ignore start - edge case: path with only separators */
    if (parts.length === 0) return null
    /* v8 ignore stop */

    return parts.slice(0, -1).join('/')
  }

  private shouldScanDirectory(name: string): boolean {
    const skipDirs = [
      'node_modules',
      '.git',
      'dist',
      'build',
      '__tests__',
      'test',
      'tests',
      'coverage',
    ]

    return !skipDirs.includes(name) && !name.startsWith('_') && !name.startsWith('.')
  }

  private sortRoutes(routes: AppRouteEntry[]): AppRouteEntry[] {
    return routes.sort((a, b) => {
      const getSpecificity = (route: AppRouteEntry): number => {
        if (!route.isDynamic) return 0

        const hasCatchAll = route.segments.some(s => s.type === 'catch-all')
        const hasOptionalCatchAll = route.segments.some(s => s.type === 'optional-catch-all')

        if (hasOptionalCatchAll) return 3
        if (hasCatchAll) return 2

        return 1
      }

      const aSpec = getSpecificity(a)
      const bSpec = getSpecificity(b)

      if (aSpec !== bSpec) return aSpec - bSpec

      const aDepth = a.path.split('/').length
      const bDepth = b.path.split('/').length
      if (aDepth !== bDepth) return bDepth - aDepth

      return a.path.localeCompare(b.path)
    })
  }

  private sortApiRoutes(routes: ApiRouteEntry[]): ApiRouteEntry[] {
    return routes.sort((a, b) => {
      if (!a.isDynamic && b.isDynamic) return -1
      if (a.isDynamic && !b.isDynamic) return 1

      const aDepth = a.path.split('/').length
      const bDepth = b.path.split('/').length
      /* v8 ignore start - depth comparison edge case */
      if (aDepth !== bDepth) return aDepth - bDepth
      /* v8 ignore stop */

      return a.path.localeCompare(b.path)
    })
  }

  private sortLayouts(layouts: LayoutEntry[]): LayoutEntry[] {
    return layouts.sort((a, b) => {
      /* v8 ignore start - root layout sorting comparisons */
      if (a.path === '/' && b.path !== '/') return -1
      if (b.path === '/' && a.path !== '/') return 1
      /* v8 ignore stop */

      const aDepth = a.path.split('/').length
      const bDepth = b.path.split('/').length
      return aDepth - bDepth
    })
  }

  private sortTemplates(templates: TemplateEntry[]): TemplateEntry[] {
    return templates.sort((a, b) => {
      /* v8 ignore start - root template sorting comparisons */
      if (a.path === '/' && b.path !== '/') return -1
      if (b.path === '/' && a.path !== '/') return 1
      /* v8 ignore stop */

      const aDepth = a.path.split('/').length
      const bDepth = b.path.split('/').length
      return aDepth - bDepth
    })
  }

  private async detectHttpMethods(filePath: string): Promise<string[]> {
    const fullPath = path.join(this.appDir, filePath)
    const content = await fs.readFile(fullPath, 'utf-8')
    const methods: string[] = []

    for (const method of HTTP_METHODS) {
      const functionExportRegex = new RegExp(`export\\s+(?:async\\s+)?function\\s+${method}\\s*\\(`)
      const constExportRegex = new RegExp(
        `export\\s+(?:async\\s+)?(?:const|let|var)\\s+${method}\\s*=`,
      )

      if (functionExportRegex.test(content) || constExportRegex.test(content)) methods.push(method)
    }

    return methods
  }

  private async processApiRouteFile(
    relativePath: string,
    fileName: string,
  ): Promise<ApiRouteEntry> {
    const filePath = toPosixPath(path.join(relativePath, fileName))
    const routePath = this.pathToRoute(relativePath)
    const segments = this.parseRouteSegments(relativePath)
    const params = this.extractParams(segments)
    const methods = await this.detectHttpMethods(filePath)

    return {
      path: routePath,
      filePath,
      segments,
      params,
      isDynamic: params.length > 0,
      methods,
    }
  }
}

export async function generateAppRouteManifest(
  appDir: string,
  options: Readonly<Partial<AppRouteGeneratorOptions>> = {},
): Promise<AppRouteManifest> {
  const generator = new AppRouteGenerator({
    appDir,
    ...options,
  })

  return generator.generateManifest()
}
