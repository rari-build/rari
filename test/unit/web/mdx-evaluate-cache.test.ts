import type { ComponentType } from 'react'
import { describe, expect, it, vi } from 'vite-plus/test'
import {
  resetMdxEvaluateCacheForTests,
  withMdxEvaluateCache,
} from '../../../web/src/lib/mdx/evaluate-cached'

function stubComponent(): ComponentType {
  return (() => null) as ComponentType
}

describe('withMdxEvaluateCache', () => {
  it('evaluates once per filePath+content and reuses the result', async () => {
    resetMdxEvaluateCacheForTests()
    const load = vi.fn(async () => {
      await Promise.resolve()
      return stubComponent()
    })

    const first = await withMdxEvaluateCache('docs/a.mdx', '# hi', load)
    const second = await withMdxEvaluateCache('docs/a.mdx', '# hi', load)

    expect(load).toHaveBeenCalledOnce()
    expect(second).toBe(first)
  })

  it('re-evaluates when content changes', async () => {
    resetMdxEvaluateCacheForTests()
    const load = vi.fn(async () => {
      await Promise.resolve()
      return stubComponent()
    })

    await withMdxEvaluateCache('docs/a.mdx', '# one', load)
    await withMdxEvaluateCache('docs/a.mdx', '# two', load)

    expect(load).toHaveBeenCalledTimes(2)
  })

  it('dedupes concurrent loads for the same key', async () => {
    resetMdxEvaluateCacheForTests()
    const deferred = Promise.withResolvers<ComponentType>()
    const load = vi.fn(async () => {
      const component = await deferred.promise
      return component
    })

    const a = withMdxEvaluateCache('docs/a.mdx', '# hi', load)
    const b = withMdxEvaluateCache('docs/a.mdx', '# hi', load)
    expect(load).toHaveBeenCalledOnce()

    const component = stubComponent()
    deferred.resolve(component)
    expect(await a).toBe(component)
    expect(await b).toBe(component)
  })
})
