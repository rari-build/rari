import { afterEach, beforeEach, vi } from 'vite-plus/test'

beforeEach(() => {
  vi.clearAllMocks()
})

afterEach(() => {
  vi.restoreAllMocks()
})
