export interface PendingScrollToTop<T extends object> {
  readonly payload: T
  readonly commitKey: number
}

export function resolvePendingScrollToTop<T extends object>(
  pending: PendingScrollToTop<T> | null,
  committedPayload: T | undefined,
  renderKey: number,
): { readonly shouldScroll: boolean; readonly nextPending: PendingScrollToTop<T> | null } {
  if (pending == null) return { shouldScroll: false, nextPending: null }
  if (pending.commitKey !== renderKey) return { shouldScroll: false, nextPending: null }
  if (committedPayload !== pending.payload) return { shouldScroll: false, nextPending: pending }
  return { shouldScroll: true, nextPending: null }
}
