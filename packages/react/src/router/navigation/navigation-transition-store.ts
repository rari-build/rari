interface NavigationTransitionSnapshot {
  readonly generation: number
  readonly pathname: string
  readonly search: string
}

let generation = 0
let pathname = '/'
let search = ''
let initializedFromWindow = false
let cachedSnapshot: NavigationTransitionSnapshot = { generation, pathname, search }
let serverSnapshotCache: NavigationTransitionSnapshot | null = null
const listeners = new Set<() => void>()

function refreshCachedSnapshot(): void {
  cachedSnapshot = { generation, pathname, search }
}

function emit(): void {
  refreshCachedSnapshot()
  for (const listener of listeners) listener()
}

function ensureInitializedFromWindow(): void {
  if (initializedFromWindow || typeof window === 'undefined') return
  initializedFromWindow = true
  pathname = window.location.pathname
  search = window.location.search
  refreshCachedSnapshot()
}

export function getNavigationTransitionSnapshot(): NavigationTransitionSnapshot {
  ensureInitializedFromWindow()
  return cachedSnapshot
}

export function getNavigationTransitionServerSnapshot(
  initialPathname = '/',
): NavigationTransitionSnapshot {
  if (
    serverSnapshotCache?.pathname === initialPathname &&
    serverSnapshotCache.search === '' &&
    serverSnapshotCache.generation === 0
  ) {
    return serverSnapshotCache
  }
  serverSnapshotCache = { generation: 0, pathname: initialPathname, search: '' }
  return serverSnapshotCache
}

export function subscribeNavigationTransition(onStoreChange: () => void): () => void {
  listeners.add(onStoreChange)
  return () => {
    listeners.delete(onStoreChange)
  }
}

export function publishNavigationTransition(next: {
  readonly pathname: string
  readonly search?: string
}): void {
  initializedFromWindow = true
  generation += 1
  pathname = next.pathname
  search = next.search ?? ''
  emit()
}

export function syncNavigationTransitionFromWindow(): void {
  if (typeof window === 'undefined') return
  initializedFromWindow = true
  const nextPathname = window.location.pathname
  const nextSearch = window.location.search
  if (nextPathname === pathname && nextSearch === search) return
  pathname = nextPathname
  search = nextSearch
  emit()
}
