import { expect, test } from '@playwright/test'

test.describe('Node APIs', () => {
  test('exercises core node:* APIs used by rari apps', async ({ request }) => {
    const response = await request.get('/api/node-apis')
    expect(response.status()).toBe(200)

    const data = await response.json()
    expect(data.ok, `failed probes: ${JSON.stringify(data.failed)}`).toBe(true)
    expect(data.failed).toEqual([])

    expect(data.probes.process.cwd).toBe(true)
    expect(data.probes.fs.readPackageName).toBe(true)
    expect(data.probes.crypto.sha256).toBe(true)
    expect(data.probes.asyncHooks.asyncLocalStorage).toBe(true)
  })
})
