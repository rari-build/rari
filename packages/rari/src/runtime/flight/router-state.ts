export type FlightRouterState = readonly [
  segment: string,
  parallelRoutes: { readonly children?: FlightRouterState },
]

export type SegmentPath = readonly string[]

const CHILDREN_SLOT = 'children'

function trimTrailingSlashes(pathname: string): string {
  let end = pathname.length
  while (end > 1 && pathname[end - 1] === '/') end -= 1

  return pathname.slice(0, end)
}

export function buildFlightRouterState(pathname: string): FlightRouterState {
  const normalized =
    pathname === '/' || pathname === '' ? '/' : trimTrailingSlashes(pathname) || '/'

  const segments = normalized === '/' ? [] : normalized.slice(1).split('/').filter(Boolean)

  let leaf: FlightRouterState = ['', {}]
  for (let index = segments.length - 1; index >= 0; index -= 1) {
    leaf = [segments[index], { [CHILDREN_SLOT]: leaf }]
  }

  return ['', { [CHILDREN_SLOT]: leaf }]
}

export function segmentPathFromRouterState(tree: FlightRouterState): SegmentPath {
  const path: string[] = []
  let current: FlightRouterState | undefined = tree

  while (current) {
    const segment: string = current[0]
    const parallelRoutes: { children?: FlightRouterState } = current[1]
    if (segment) path.push(segment)

    current = parallelRoutes.children
  }

  return path
}

export function segmentPathFromPathname(pathname: string): SegmentPath {
  return segmentPathFromRouterState(buildFlightRouterState(pathname))
}

export function pathnameFromSegmentPath(segmentPath: SegmentPath): string {
  if (segmentPath.length === 0) return '/'

  return `/${segmentPath.join('/')}`
}
