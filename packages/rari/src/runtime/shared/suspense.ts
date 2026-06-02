export const SUSPENSE_TYPES: readonly string[] = [
  '$Sreact.suspense',
  'react.suspense',
  'suspense',
  'Suspense',
]

export function isSuspenseType(type: unknown): boolean {
  return typeof type === 'string' && SUSPENSE_TYPES.includes(type)
}
