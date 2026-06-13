import process from 'node:process'
import { defineConfig, devices } from '@playwright/test'

import { getRariLogPath } from './test/e2e/shared/helpers'

export default defineConfig({
  testDir: './test/e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? 'github' : 'html',

  use: {
    baseURL: process.env.BASE_URL || `http://localhost:${process.env.PORT || 3000}`,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },

  webServer: {
    command: `pnpm --filter @test/app build && pnpm --filter @test/app start > "${getRariLogPath()}" 2>&1`,
    url: `http://localhost:${process.env.PORT || 3000}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
    env: {
      RUST_LOG: 'debug',
      RARI_REVALIDATE_SECRET: 'e2e-test-secret',
    },
  },

  projects: [
    {
      name: 'Mobile Chrome',
      use: {
        ...devices['Pixel 5'],
        launchOptions: {
          slowMo: process.env.SLOW_MO ? Number(process.env.SLOW_MO) || 0 : 0,
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
