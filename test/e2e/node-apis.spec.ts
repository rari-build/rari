import { expect, test } from '@playwright/test'

test.describe('Node APIs', () => {
  test('exercises core node:* APIs used by rari apps', async ({ request }) => {
    const response = await request.get('/api/node-apis')
    expect(response.status()).toBe(200)

    const data: unknown = await response.json()
    expect(data).toMatchObject({
      ok: true,
      failed: [],
      probes: {
        process: { cwd: true },
        fs: { readPackageName: true },
        crypto: { sha256: true },
        asyncHooks: { asyncLocalStorage: true },
      },
    })
  })
})
