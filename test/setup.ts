import { afterEach, beforeEach, vi } from 'vite-plus/test'

if (!('g' in globalThis)) {
  Object.defineProperty(globalThis, 'g', {
    value: globalThis,
    writable: false,
    enumerable: false,
    configurable: true,
  })
}

beforeEach(() => {
  vi.clearAllMocks()
})

afterEach(() => {
  vi.restoreAllMocks()
})
