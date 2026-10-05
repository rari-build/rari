import { rariQwik } from '@rari/qwik/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    // <Pending> + out-of-order streaming (Qwik 2 experimental flag).
    rariQwik({ experimental: ['pendingBoundary'] }),
  ],
})
