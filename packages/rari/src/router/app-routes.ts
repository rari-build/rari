import type {
  ApiRouteEntry,
  AppRouteEntry,
  AppRouteManifest,
  ErrorEntry,
  LayoutEntry,
  LoadingEntry,
  NotFoundEntry,
  RouteSegment,
  RouteSegmentType,
} from './app-types'
import { promises as fs } from 'node:fs'
import path from 'node:path'

export interface AppRouteGeneratorOptions {
  appDir: string
  extensions?: string[]
  verbose?: boolean
}

const SPECIAL_FILES = {
  PAGE: 'page',
  LAYOUT: 'layout',
  LOADING: 'loading',
  ERROR: 'error',
  NOT_FOUND: 'not-found',
  TEMPLATE: 'template',
  DEFAULT: 'default',
  ROUTE: 'route',
} as const

const SEGMENT_PATTERNS = {
  DYNAMIC: /^\[([^\]]+)\]$/,
  CATCH_ALL: /^\[\.\.\.([^\]]+)\]$/,
  OPTIONAL_CATCH_ALL: /^\[\[\.\.\.([^\]]+)\]\]$/,
} as const

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'] as const

export class AppRouteGenerator {
  private appDir: string
  private extensions: string[]
  private verbose: boolean

  constructor(options: AppRouteGeneratorOptions) {
    this.appDir = path.resolve(options.appDir)
    this.extensions = options.extensions || ['.tsx', '.jsx', '.ts', '.js']
    this.verbose = options.verbose || false
  }

  async generateManifest(): Promise<AppRouteManifest> {
    if (this.verbose) {
      console.warn(`[AppRouter] Scanning app directory: ${this.appDir}`)
    }

    const routes: AppRouteEntry[] = []
    const layouts: LayoutEntry[] = []
    const loading: LoadingEntry[] = []
    const errors: ErrorEntry[] = []
    const notFound: NotFoundEntry[] = []
    const apiRoutes: ApiRouteEntry[] = []

    await this.scanDirectory('', routes, layouts, loading, errors, notFound, apiRoutes)

    if (this.verbose) {
      console.warn(`[AppRouter] Found ${routes.length} routes`)
      console.warn(`[AppRouter] Found ${layouts.length} layouts`)
      console.warn(`[AppRouter] Found ${loading.length} loading components`)
      console.warn(`[AppRouter] Found ${errors.length} error boundaries`)
      console.warn(`[AppRouter] Found ${apiRoutes.length} API routes`)
    }

    return {
      routes: this.sortRoutes(routes),
      layouts: this.sortLayouts(layouts),
      loading,
      errors,
      notFound,
      apiRoutes: this.sortApiRoutes(apiRoutes),
      generated: new Date().toISOString(),
    }
  }

  private async scanDirectory(
    relativePath: string,
    routes: AppRouteEntry[],
    layouts: LayoutEntry[],
    loading: LoadingEntry[],
    errors: ErrorEntry[],
    notFound: NotFoundEntry[],
    apiRoutes: ApiRouteEntry[],
  ): Promise<void> {
    const fullPath = path.join(this.appDir, relativePath)

    let entries: string[]
    try {
      entries = await fs.readdir(fullPath)
    }
    catch {
      return
    }

    const files: string[] = []
    const dirs: string[] = []

    for (const entry of entries) {
      const entryPath = path.join(fullPath, entry)
      const stat = await fs.stat(entryPath)

      if (stat.isDirectory()) {
        if (this.shouldScanDirectory(entry)) {
          dirs.push(entry)
        }
      }
      else if (stat.isFile()) {
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
      apiRoutes,
    )

    for (const dir of dirs) {
      const subPath = relativePath ? path.join(relativePath, dir) : dir
      await this.scanDirectory(subPath, routes, layouts, loading, errors, notFound, apiRoutes)
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
    apiRoutes: ApiRouteEntry[],
  ): Promise<void> {
    const routePath = this.pathToRoute(relativePath)

    const pageFile = this.findFile(files, SPECIAL_FILES.PAGE)
    if (pageFile) {
      const segments = this.parseRouteSegments(relativePath)
      const params = this.extractParams(segments)

      routes.push({
        path: routePath,
        filePath: path.join(relativePath, pageFile),
        segments,
        params,
        isDynamic: params.length > 0,
      })
    }

    const layoutFile = this.findFile(files, SPECIAL_FILES.LAYOUT)
    if (layoutFile) {
      const parentPath = this.getParentPath(relativePath)
      layouts.push({
        path: routePath,
        filePath: path.join(relativePath, layoutFile),
        parentPath: parentPath ? this.pathToRoute(parentPath) : undefined,
      })
    }

    const loadingFile = this.findFile(files, SPECIAL_FILES.LOADING)
    if (loadingFile) {
      loading.push({
        path: routePath,
        filePath: path.join(relativePath, loadingFile),
      })
    }

    const errorFile = this.findFile(files, SPECIAL_FILES.ERROR)
    if (errorFile) {
      errors.push({
        path: routePath,
        filePath: path.join(relativePath, errorFile),
      })
    }

    const notFoundFile = this.findFile(files, SPECIAL_FILES.NOT_FOUND)
    if (notFoundFile) {
      notFound.push({
        path: routePath,
        filePath: path.join(relativePath, notFoundFile),
      })
    }

    const routeFile = this.findFile(files, SPECIAL_FILES.ROUTE)
    if (routeFile) {
      const apiRoute = await this.processApiRouteFile(relativePath, routeFile)
      apiRoutes.push(apiRoute)
    }
  }

  private findFile(files: string[], baseName: string): string | undefined {
    for (const ext of this.extensions) {
      const fileName = `${baseName}${ext}`
      if (files.includes(fileName)) {
        return fileName
      }
    }
    return undefined
  }

  private pathToRoute(filePath: string): string {
    if (!filePath) {
      return '/'
    }

    const normalized = filePath.replace(/\\/g, '/')

    const segments = normalized.split('/').filter(Boolean)
    const routeSegments = segments.map((segment) => {
      if (SEGMENT_PATTERNS.OPTIONAL_CATCH_ALL.test(segment)) {
        const match = segment.match(SEGMENT_PATTERNS.OPTIONAL_CATCH_ALL)
        return `[[...${match![1]}]]`
      }
      if (SEGMENT_PATTERNS.CATCH_ALL.test(segment)) {
        const match = segment.match(SEGMENT_PATTERNS.CATCH_ALL)
        return `[...${match![1]}]`
      }
      if (SEGMENT_PATTERNS.DYNAMIC.test(segment)) {
        const match = segment.match(SEGMENT_PATTERNS.DYNAMIC)
        return `[${match![1]}]`
      }
      return segment
    })

    return `/${routeSegments.join('/')}`
  }

  private parseRouteSegments(filePath: string): RouteSegment[] {
    if (!filePath) {
      return []
    }

    const segments = filePath.split(/[/\\]/).filter(Boolean)
    return segments.map((segment) => {
      if (SEGMENT_PATTERNS.OPTIONAL_CATCH_ALL.test(segment)) {
        const match = segment.match(SEGMENT_PATTERNS.OPTIONAL_CATCH_ALL)
        return {
          type: 'optional-catch-all' as RouteSegmentType,
          value: segment,
          param: match![1],
        }
      }

      if (SEGMENT_PATTERNS.CATCH_ALL.test(segment)) {
        const match = segment.match(SEGMENT_PATTERNS.CATCH_ALL)
        return {
          type: 'catch-all' as RouteSegmentType,
          value: segment,
          param: match![1],
        }
      }

      if (SEGMENT_PATTERNS.DYNAMIC.test(segment)) {
        const match = segment.match(SEGMENT_PATTERNS.DYNAMIC)
        return {
          type: 'dynamic' as RouteSegmentType,
          value: segment,
          param: match![1],
        }
      }

      return {
        type: 'static' as RouteSegmentType,
        value: segment,
      }
    })
  }

  private extractParams(segments: RouteSegment[]): string[] {
    return segments
      .filter(seg => seg.param !== undefined)
      .map(seg => seg.param!)
  }

  private getParentPath(filePath: string): string | null {
    if (!filePath) {
      return null
    }
    const parts = filePath.split(/[/\\]/).filter(Boolean)
    if (parts.length === 0) {
      return null
    }
    return parts.slice(0, -1).join('/')
  }

  async findLayoutChain(routePath: string, manifest: AppRouteManifest): Promise<LayoutEntry[]> {
    const chain: LayoutEntry[] = []
    const segments = routePath.split('/').filter(Boolean)

    for (let i = segments.length; i >= 0; i--) {
      const currentPath = i === 0 ? '/' : `/${segments.slice(0, i).join('/')}`
      const layout = manifest.layouts.find(l => l.path === currentPath)
      if (layout) {
        chain.unshift(layout)
      }
    }

    return chain
  }

  private shouldScanDirectory(name: string): boolean {
    const skipDirs = [
      'node_modules',
      '.git',
      '.next',
      'dist',
      'build',
      '__tests__',
      'test',
      'tests',
      'coverage',
    ]

    return !skipDirs.includes(name) && !name.startsWith('.')
  }

  private sortRoutes(routes: AppRouteEntry[]): AppRouteEntry[] {
    return routes.sort((a, b) => {
      if (!a.isDynamic && b.isDynamic)
        return -1
      if (a.isDynamic && !b.isDynamic)
        return 1

      const aDepth = a.path.split('/').length
      const bDepth = b.path.split('/').length
      if (aDepth !== bDepth)
        return aDepth - bDepth

      return a.path.localeCompare(b.path)
    })
  }

  private sortApiRoutes(routes: ApiRouteEntry[]): ApiRouteEntry[] {
    return routes.sort((a, b) => {
      if (!a.isDynamic && b.isDynamic)
        return -1
      if (a.isDynamic && !b.isDynamic)
        return 1

      const aDepth = a.path.split('/').length
      const bDepth = b.path.split('/').length
      if (aDepth !== bDepth)
        return aDepth - bDepth

      return a.path.localeCompare(b.path)
    })
  }

  private sortLayouts(layouts: LayoutEntry[]): LayoutEntry[] {
    return layouts.sort((a, b) => {
      if (a.path === '/' && b.path !== '/')
        return -1
      if (b.path === '/' && a.path !== '/')
        return 1

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
      const functionExportRegex = new RegExp(
        `export\\s+(?:async\\s+)?function\\s+${method}\\s*\\(`,
      )
      const constExportRegex = new RegExp(
        `export\\s+(?:async\\s+)?(?:const|let|var)\\s+${method}\\s*=`,
      )

      if (functionExportRegex.test(content) || constExportRegex.test(content)) {
        methods.push(method)
      }
    }

    return methods
  }

  private async processApiRouteFile(
    relativePath: string,
    fileName: string,
  ): Promise<ApiRouteEntry> {
    const filePath = path.join(relativePath, fileName)
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
  options: Partial<AppRouteGeneratorOptions> = {},
): Promise<AppRouteManifest> {
  const generator = new AppRouteGenerator({
    appDir,
    ...options,
  })

  return generator.generateManifest()
}

export async function writeManifest(
  manifest: AppRouteManifest,
  outputPath: string,
): Promise<void> {
  const json = JSON.stringify(manifest, null, 2)
  await fs.writeFile(outputPath, json, 'utf-8')
}

export async function loadManifest(manifestPath: string): Promise<AppRouteManifest> {
  const content = await fs.readFile(manifestPath, 'utf-8')
  return JSON.parse(content)
}
