import { expect, test } from '@playwright/test'

import { NODE_BUILTIN_MODULES } from '../fixtures/app/src/lib/node-builtins'

test.describe('Node builtins', () => {
  test('lists the deno_node builtin inventory', async ({ request }) => {
    const response = await request.get('/api/node-builtins')
    expect(response.status()).toBe(200)

    const data = await response.json()
    expect(data.total).toBe(NODE_BUILTIN_MODULES.length)
    expect(data.modules).toEqual([...NODE_BUILTIN_MODULES])
  })

  test('resolves every deno_node builtin module', async ({ request }) => {
    const failed: Array<{ name: string, error?: string }> = []

    for (const name of NODE_BUILTIN_MODULES) {
      const response = await request.get(
        `/api/node-builtins?name=${encodeURIComponent(name)}`,
      )
      expect(response.status(), name).toBe(200)

      const data = await response.json() as { name: string, ok: boolean, error?: string }
      if (!data.ok)
        failed.push({ name: data.name, error: data.error })
    }

    expect(failed, `failed imports:\n${JSON.stringify(failed, null, 2)}`).toEqual([])
  })
})
