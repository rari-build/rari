import type { FlightRouterState } from './router-state'
import { buildFlightRouterState } from './router-state'

export interface RariRouterState {
  pathname: string
  search: string
  tree: FlightRouterState
}

export function getRouterState(): RariRouterState {
  if (typeof window === 'undefined') {
    return {
      pathname: '/',
      search: '',
      tree: buildFlightRouterState('/'),
    }
  }

  const pathname = window.location.pathname
  const search = window.location.search

  return {
    pathname,
    search,
    tree: buildFlightRouterState(pathname),
  }
}

export function serializeRouterState(): string {
  return JSON.stringify(getRouterState())
}
