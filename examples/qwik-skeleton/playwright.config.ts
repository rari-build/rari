import { defineConfig, devices } from '@playwright/test'

// Conformance harness for the rari Qwik adapter: builds the app with the
// rariQwik Vite plugin, serves it through the rari binary (debug build), and
// runs ported Qwik behaviour specs against it.
export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'pnpm build && pnpm start',
    url: 'http://localhost:3000/',
    reuseExistingServer: false,
    timeout: 180_000,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
})
