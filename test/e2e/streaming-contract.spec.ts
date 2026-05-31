import { expect, test } from '@playwright/test'

test.describe('Streaming contract endpoint', () => {
  test('streams the first RSC chunk before delayed work completes', async ({ baseURL }) => {
    if (!baseURL) {
      throw new Error('baseURL must be configured for streaming contract tests')
    }

    const endpoint = new URL('/_rari/streaming-contract', baseURL).toString()
    const startedAt = Date.now()

    const response = await fetch(endpoint, { method: 'POST' })

    expect(response.ok).toBe(true)
    expect(response.headers.get('content-type')).toContain('text/x-component')
    expect(response.headers.get('x-render-mode')).toBe('streaming-contract')
    expect(response.body).not.toBeNull()

    const reader = response.body!.getReader()
    const decoder = new TextDecoder()

    const first = await reader.read()
    const firstChunkAt = Date.now() - startedAt

    expect(first.done).toBe(false)
    const firstText = decoder.decode(first.value, { stream: true })

    expect(firstChunkAt).toBeLessThan(800)
    expect(firstText).toContain('streaming shell')
    expect(firstText).not.toContain('slow server content')

    let restText = ''
    let slowChunkAt: number | null = null

    while (true) {
      const chunk = await reader.read()
      if (chunk.done) {
        break
      }

      const text = decoder.decode(chunk.value, { stream: true })
      restText += text

      if (slowChunkAt === null && restText.includes('slow server content')) {
        slowChunkAt = Date.now() - startedAt
      }
    }

    restText += decoder.decode()

    expect(restText).toContain('slow server content')
    expect(slowChunkAt).not.toBeNull()
    expect(firstChunkAt).toBeLessThan(slowChunkAt!)
  })
})
