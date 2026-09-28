import { isNotFoundError, NOT_FOUND_ERROR_DIGEST, notFound } from '@rari/navigation/not-found'
import { describe, expect, it } from 'vite-plus/test'

describe('notFound', () => {
  it('throws an error with the not-found digest', () => {
    expect(() => notFound()).toThrow(NOT_FOUND_ERROR_DIGEST)

    let thrown: unknown
    try {
      notFound()
    } catch (error) {
      thrown = error
    }

    expect(isNotFoundError(thrown)).toBe(true)
    expect(thrown).toMatchObject({ digest: NOT_FOUND_ERROR_DIGEST })
  })

  it('does not treat unrelated errors as not-found', () => {
    expect(isNotFoundError(new Error('boom'))).toBe(false)
    expect(isNotFoundError(null)).toBe(false)
    expect(isNotFoundError({ digest: 'other' })).toBe(false)
  })
})
