import { expect, test } from '@playwright/test'
import { NODE_BUILTIN_MODULES } from '../fixtures/app/src/lib/node-builtins'

function isFailedProbe(value: unknown): value is { name: string; ok: false; error?: string } {
  return (
    typeof value === 'object' &&
    value !== null &&
    'ok' in value &&
    value.ok === false &&
    'name' in value &&
    typeof value.name === 'string'
  )
}

test.describe('Node builtins', () => {
  test('lists the deno_node builtin inventory', async ({ request }) => {
    const response = await request.get('/api/node-builtins')
    expect(response.status()).toBe(200)

    const data: unknown = await response.json()
    expect(data).toEqual({
      total: NODE_BUILTIN_MODULES.length,
      modules: [...NODE_BUILTIN_MODULES],
    })
  })

  test('resolves every deno_node builtin module', async ({ request }) => {
    const failed: Array<{ name: string; error?: string }> = []

    for (const name of NODE_BUILTIN_MODULES) {
      const response = await request.get(`/api/node-builtins?name=${encodeURIComponent(name)}`)
      expect(response.status(), name).toBe(200)

      const data: unknown = await response.json()
      if (isFailedProbe(data)) failed.push({ name: data.name, error: data.error })
    }

    expect(failed, `failed imports:\n${JSON.stringify(failed, null, 2)}`).toEqual([])
  })
})
