import path from 'node:path'

export type AppRouterFileType = 'page' | 'layout' | 'loading' | 'error' | 'not-found'

export interface AppRouterFileInfo {
  type: AppRouterFileType
  routePath: string
  filePath: string
  affectedRoutes: string[]
}

const SPECIAL_FILE_NAMES = {
  'page': ['page.tsx', 'page.ts', 'page.jsx', 'page.js'],
  'layout': ['layout.tsx', 'layout.ts', 'layout.jsx', 'layout.js'],
  'loading': ['loading.tsx', 'loading.ts', 'loading.jsx', 'loading.js'],
  'error': ['error.tsx', 'error.ts', 'error.jsx', 'error.js'],
  'not-found': ['not-found.tsx', 'not-found.ts', 'not-found.jsx', 'not-found.js'],
} as const

export function getAppRouterFileType(filePath: string): AppRouterFileType | null {
  const fileName = path.basename(filePath)

  for (const [type, fileNames] of Object.entries(SPECIAL_FILE_NAMES)) {
    if ((fileNames as readonly string[]).includes(fileName)) {
      return type as AppRouterFileType
    }
  }

  return null
}

export function extractRoutePathFromFile(filePath: string, appDir: string = 'app'): string {
  const normalized = filePath.replace(/\\/g, '/')

  const appDirPattern = `/${appDir}/`
  const appDirIndex = normalized.indexOf(appDirPattern)
  if (appDirIndex === -1) {
    const parts = normalized.split('/')
    const appIndex = parts.indexOf(appDir)
    if (appIndex !== -1 && appIndex === parts.length - 2) {
      return '/'
    }
    return '/'
  }

  const afterAppDir = normalized.substring(appDirIndex + appDirPattern.length)

  const dirPath = path.dirname(afterAppDir)

  if (dirPath === '.' || dirPath === '') {
    return '/'
  }

  const segments = dirPath.split('/').filter(Boolean)

  if (segments.length === 0) {
    return '/'
  }

  return `/${segments.join('/')}`
}

export function determineAffectedRoutes(
  filePath: string,
  fileType: AppRouterFileType,
  appDir: string = 'app',
  allRoutes: string[] = [],
): string[] {
  const routePath = extractRoutePathFromFile(filePath, appDir)

  if (fileType !== 'layout') {
    return [routePath]
  }

  const affectedRoutes = [routePath]

  if (allRoutes.length > 0) {
    for (const route of allRoutes) {
      if (route !== routePath && route.startsWith(routePath)) {
        if (routePath === '/') {
          affectedRoutes.push(route)
        }
        else if (route.startsWith(`${routePath}/`)) {
          affectedRoutes.push(route)
        }
      }
    }
  }

  return affectedRoutes
}

export function isAppRouterFile(filePath: string): boolean {
  return getAppRouterFileType(filePath) !== null
}

export function getAppRouterFileInfo(
  filePath: string,
  appDir: string = 'app',
  allRoutes: string[] = [],
): AppRouterFileInfo | null {
  const fileType = getAppRouterFileType(filePath)

  if (!fileType) {
    return null
  }

  const routePath = extractRoutePathFromFile(filePath, appDir)
  const affectedRoutes = determineAffectedRoutes(filePath, fileType, appDir, allRoutes)

  return {
    type: fileType,
    routePath,
    filePath,
    affectedRoutes,
  }
}
