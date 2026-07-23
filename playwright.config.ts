import process from 'node:process'
import { defineConfig, devices } from '@playwright/test'
import { getRariLogPath } from './test/e2e/shared/helpers'

function env(name: string): string | undefined {
  const value = process.env[name]
  return typeof value === 'string' && value !== '' ? value : undefined
}

const port = env('PORT') ?? '3000'
const baseURL = env('BASE_URL') ?? `http://localhost:${port}`
const isCI = env('CI') != null

export default defineConfig({
  testDir: './test/e2e',
  fullyParallel: true,
  forbidOnly: isCI,
  retries: isCI ? 2 : 0,
  workers: (() => {
    const parsed = Number(env('E2E_WORKERS'))
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 1
  })(),
  reporter: isCI ? 'github' : 'html',

  use: {
    baseURL,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },

  webServer: {
    command: `pnpm --filter rari build && pnpm --filter @test/app build && pnpm --filter @test/app start > "${getRariLogPath()}" 2>&1`,
    url: `http://localhost:${port}`,
    // Reusing a running server after `pnpm build` serves stale SSR HTML with outdated
    // asset hashes (client JS 404s). Set E2E_REUSE_SERVER=1 to opt in locally.
    reuseExistingServer: env('E2E_REUSE_SERVER') === '1',
    timeout: 120000,
    env: {
      NODE_ENV: 'production',
      RUST_LOG: 'debug',
      RARI_REVALIDATE_SECRET: 'e2e-test-secret',
    },
  },

  projects: [
    {
      name: 'Mobile Chrome',
      testIgnore: '**/cache-logs.spec.ts',
      use: {
        ...devices['Pixel 5'],
        launchOptions: {
          slowMo: (() => {
            const value = env('SLOW_MO')
            if (value == null) return 0
            const parsed = Number(value)
            return Number.isFinite(parsed) ? parsed : 0
          })(),
        },
      },
    },
    {
      name: 'Desktop Chrome',
      use: {
        ...devices['Desktop Chrome'],
      },
    },
  ],
})
