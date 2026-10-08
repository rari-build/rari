export const NOT_FOUND_ERROR_DIGEST = 'RARI_NOT_FOUND'

export function isNotFoundError(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'digest' in error &&
    (error as { digest?: unknown }).digest === NOT_FOUND_ERROR_DIGEST
  )
}

export function notFound(): never {
  const error = new Error(NOT_FOUND_ERROR_DIGEST)
  ;(error as Error & { digest?: string }).digest = NOT_FOUND_ERROR_DIGEST
  throw error
}
