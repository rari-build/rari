import { createContext, use } from 'react'

export interface PendingLoadingLeaf {
  readonly pathname: string
  readonly search: string
}

export const PendingLoadingLeafContext = createContext<PendingLoadingLeaf | null>(null)

export function usePendingLoadingLeaf(): PendingLoadingLeaf | null {
  return use(PendingLoadingLeafContext)
}

export function isPendingLoadingLeaf(
  pathname: string,
  search: string,
  pending: PendingLoadingLeaf | null,
): boolean {
  return pending?.pathname === pathname && pending.search === search
}
