export function resolvePendingScrollToTop<T extends object>(
  pendingPayload: T | null,
  committedPayload: T | undefined,
): { readonly shouldScroll: boolean; readonly nextPending: T | null } {
  if (pendingPayload == null) return { shouldScroll: false, nextPending: null }
  if (committedPayload !== pendingPayload)
    return { shouldScroll: false, nextPending: pendingPayload }
  return { shouldScroll: true, nextPending: null }
}
