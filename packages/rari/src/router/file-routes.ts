import type { Route, RouteGenerationOptions } from './types'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import chokidar from 'chokidar'
import {
  analyzeFilePath,
} from './utils'

interface RouteTreeNode {
  route: Route
  children: Map<string, RouteTreeNode>
  layout?: Route
}

export class FileRouteGenerator {
  private pagesDir: string
  private extensions: string[]
  private routes: Route[] = []
  private routeTree: RouteTreeNode | null = null

  constructor(options: RouteGenerationOptions) {
    this.pagesDir = options.pagesDir
    this.extensions = options.extensions || ['.tsx', '.jsx', '.ts', '.js']
  }

  async generateRoutes(): Promise<Route[]> {
    const files = await this.scanPagesDirectory()
    const routes = await this.processFiles(files)

    this.routeTree = this.buildRouteTree(routes)
    this.routes = this.flattenRouteTree(this.routeTree)

    return this.routes
  }

  private async scanPagesDirectory(): Promise<string[]> {
    const files: string[] = []

    try {
      await this.scanDirectory(this.pagesDir, files)
    }
    catch (error) {
      console.warn(`Failed to scan pages directory: ${this.pagesDir}`, error)
      return []
    }

    return files.filter(file =>
      this.extensions.some(ext => file.endsWith(ext)),
    )
  }

  private async scanDirectory(dir: string, files: string[]): Promise<void> {
    let entries: string[]

    try {
      entries = await fs.readdir(dir)
    }
    catch {
      return
    }

    for (const entry of entries) {
      const fullPath = path.join(dir, entry)
      const relativePath = path.relative(this.pagesDir, fullPath)

      try {
        const stat = await fs.stat(fullPath)

        if (stat.isDirectory()) {
          if (this.shouldSkipDirectory(entry)) {
            continue
          }

          await this.scanDirectory(fullPath, files)
        }
        else if (stat.isFile()) {
          if (this.shouldSkipFile(entry)) {
            continue
          }

          files.push(relativePath)
        }
      }
      catch (error) {
        console.warn(`Failed to stat ${fullPath}`, error)
      }
    }
  }

  private shouldSkipDirectory(name: string): boolean {
    const skipDirs = [
      'node_modules',
      '.git',
      '.next',
      'dist',
      'build',
      '__tests__',
      '__test__',
      'test',
      'tests',
      'coverage',
      '.coverage',
      'utils',
      'lib',
      'styles',
    ]

    return skipDirs.includes(name) || name.startsWith('.')
  }

  private shouldSkipFile(name: string): boolean {
    const skipFiles = [
      '_app.tsx',
      '_app.jsx',
      '_document.tsx',
      '_document.jsx',
      '_error.tsx',
      '_error.jsx',
      'middleware.ts',
      'middleware.js',
    ]

    return (
      skipFiles.includes(name)
      || name.startsWith('.')
      || name.includes('.test.')
      || name.includes('.spec.')
      || name.includes('.stories.')
      || name.includes('.d.ts')
    )
  }

  private async processFiles(files: string[]): Promise<Route[]> {
    const routes: Route[] = []

    for (const file of files) {
      try {
        const route = await this.fileToRoute(file)
        if (route) {
          routes.push(route)
        }
      }
      catch (error) {
        console.warn(`Failed to process file ${file}`, error)
      }
    }

    return routes
  }

  private async fileToRoute(filePath: string): Promise<Route | null> {
    const fileInfo = analyzeFilePath(filePath)
    const routePath = fileInfo.routePath

    return {
      path: routePath,
      filePath,
      component: null as any,
      isDynamic: fileInfo.isDynamic,
      paramNames: fileInfo.paramNames,
      isLayout: fileInfo.isLayout,
      isIndex: fileInfo.isIndex,
      meta: {},
    }
  }

  private buildRouteTree(routes: Route[]): RouteTreeNode {
    const root: RouteTreeNode = {
      route: {
        path: '/',
        filePath: '',
        component: null,
        isDynamic: false,
        paramNames: [],
        meta: {},
      },
      children: new Map(),
    }

    const layouts = routes.filter(route => route.isLayout)
    const regularRoutes = routes.filter(route => !route.isLayout)

    for (const route of regularRoutes) {
      this.insertRouteIntoTree(root, route)
    }

    for (const layout of layouts) {
      this.applyLayoutToTree(root, layout)
    }

    this.establishParentChildRelations(root)

    return root
  }

  private insertRouteIntoTree(root: RouteTreeNode, route: Route): void {
    const segments = route.path === '/' ? [''] : route.path.split('/').filter(Boolean)
    let currentNode = root

    for (let i = 0; i < segments.length; i++) {
      const segment = segments[i]

      if (!currentNode.children.has(segment)) {
        const partialPath = segments.slice(0, i + 1).join('/')
        const newNode: RouteTreeNode = {
          route: {
            path: partialPath === '' ? '/' : `/${partialPath}`,
            filePath: '',
            component: null,
            isDynamic: segment.startsWith(':'),
            paramNames: [],
            meta: {},
          },
          children: new Map(),
        }
        currentNode.children.set(segment, newNode)
      }

      currentNode = currentNode.children.get(segment)!
    }

    currentNode.route = { ...route }
  }

  private applyLayoutToTree(root: RouteTreeNode, layout: Route): void {
    const layoutDir = path.dirname(layout.filePath)
    const segments = layoutDir === '.' || layoutDir === '' ? [''] : layoutDir.split('/').filter(Boolean)

    let currentNode = root

    for (const segment of segments) {
      if (currentNode.children.has(segment)) {
        currentNode = currentNode.children.get(segment)!
      }
      else {
        return
      }
    }

    currentNode.layout = layout
  }

  private establishParentChildRelations(node: RouteTreeNode, parent?: Route): void {
    if (node.route && parent) {
      node.route.parent = parent
    }

    const currentRoute = node.route

    for (const child of node.children.values()) {
      this.establishParentChildRelations(child, currentRoute)

      if (child.route && currentRoute) {
        if (!currentRoute.children) {
          currentRoute.children = []
        }
        currentRoute.children.push(child.route)
      }
    }
  }

  private flattenRouteTree(node: RouteTreeNode): Route[] {
    const routes: Route[] = []

    const traverse = (currentNode: RouteTreeNode) => {
      if (currentNode.route && currentNode.route.filePath) {
        routes.push(currentNode.route)
      }
      if (currentNode.layout) {
        routes.push(currentNode.layout)
      }
      for (const child of currentNode.children.values()) {
        traverse(child)
      }
    }

    traverse(node)
    return routes
  }

  getRoutes(): Route[] {
    return this.routes
  }

  getRouteByPath(path: string): Route | undefined {
    return this.routes.find(route => route.path === path)
  }

  getRouteByFilePath(filePath: string): Route | undefined {
    return this.routes.find(route => route.filePath === filePath)
  }

  async refresh(): Promise<Route[]> {
    return this.generateRoutes()
  }
}

export async function generateFileRoutes(options: RouteGenerationOptions): Promise<Route[]> {
  const generator = new FileRouteGenerator(options)
  return generator.generateRoutes()
}

export function watchFileRoutes(
  options: RouteGenerationOptions,
  onChange: (routes: Route[]) => void,
): () => void {
  const generator = new FileRouteGenerator(options)
  let isInitialized = false

  generator.generateRoutes().then((routes) => {
    onChange(routes)
    isInitialized = true
  })

  const watcher = chokidar.watch(options.pagesDir, {
    ignored: [
      /node_modules/,
      /\.git/,
      /dist/,
      /__tests__/,
      /__test__/,
      /test/,
      /tests/,
      /coverage/,
      /\.coverage/,
      /functions/,
      /utils/,
      /lib/,
      /styles/,
      /\.(test|spec|stories)\.(ts|tsx|js|jsx)$/,
      /\.d\.ts$/,
      /_app\.(ts|tsx|js|jsx)$/,
      /_document\.(ts|tsx|js|jsx)$/,
      /_error\.(ts|tsx|js|jsx)$/,
      /middleware\.(ts|js)$/,
    ],
    persistent: true,
    ignoreInitial: true,
  })

  let debounceTimer: NodeJS.Timeout | null = null
  const debouncedRegenerate = () => {
    if (debounceTimer) {
      clearTimeout(debounceTimer)
    }
    debounceTimer = setTimeout(async () => {
      if (isInitialized) {
        try {
          const routes = await generator.generateRoutes()
          onChange(routes)
        }
        catch (error) {
          console.warn('Failed to regenerate routes after file change:', error)
        }
      }
    }, 100)
  }

  watcher
    .on('add', (filePath) => {
      console.warn(`File added: ${filePath}`)
      debouncedRegenerate()
    })
    .on('change', (filePath) => {
      console.warn(`File changed: ${filePath}`)
      debouncedRegenerate()
    })
    .on('unlink', (filePath) => {
      console.warn(`File removed: ${filePath}`)
      debouncedRegenerate()
    })
    .on('error', (error) => {
      console.warn('File watcher error:', error)
    })

  return () => {
    if (debounceTimer) {
      clearTimeout(debounceTimer)
    }
    watcher.close()
  }
}

export async function createRouteManifest(
  routes: Route[],
  outputPath: string,
): Promise<void> {
  const manifest = {
    routes: routes.map(route => ({
      path: route.path,
      filePath: route.filePath,
      isDynamic: route.isDynamic,
      paramNames: route.paramNames,
      meta: route.meta,
    })),
    generated: new Date().toISOString(),
  }

  try {
    await fs.writeFile(outputPath, JSON.stringify(manifest, null, 2))
  }
  catch (error) {
    console.warn(`Failed to write route manifest to ${outputPath}`, error)
  }
}

export async function loadRouteManifest(manifestPath: string): Promise<Route[]> {
  try {
    const content = await fs.readFile(manifestPath, 'utf-8')
    const manifest = JSON.parse(content)

    return manifest.routes || []
  }
  catch (error) {
    console.warn(`Failed to load route manifest from ${manifestPath}`, error)
    return []
  }
}

export function validateRoutes(routes: Route[]): { valid: boolean, errors: string[] } {
  const errors: string[] = []
  const paths = new Set<string>()

  for (const route of routes) {
    if (paths.has(route.path)) {
      errors.push(`Duplicate route path: ${route.path}`)
    }
    paths.add(route.path)

    if (!/^\/[\w\-/:*]*$/.test(route.path)) {
      errors.push(`Invalid route path: ${route.path}`)
    }

    if (route.isDynamic && (!route.paramNames || route.paramNames.length === 0)) {
      errors.push(`Dynamic route ${route.path} has no parameter names`)
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  }
}

export function convertFilePatternToRoutePattern(pattern: string): string {
  return pattern
    .replace(/\[\.\.\.([^\]]+)\]/g, ':$1*') // [...slug] -> :slug*
    .replace(/\[([^\]]+)\]/g, ':$1') // [slug] -> :slug
    .replace(/\/index$/, '') // /index -> /
    .replace(/^$/, '/') // empty -> /
}
