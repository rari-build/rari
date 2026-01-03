import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari, rariRouter } from 'rari/vite'
import { defineConfig } from 'rolldown-vite'

export default defineConfig({
  plugins: [
    rari({
      serverBuild: {
        external: ['@shikijs/*'],
      },
    }),
    rariRouter(),
    tailwindcss(),
  ],
  build: {
    rolldownOptions: {
      output: {
        advancedChunks: {
          groups: [
            {
              name: (moduleId) => {
                if (moduleId.includes('node_modules')) {
                  if (moduleId.includes('react') || moduleId.includes('react-dom')) {
                    return 'vendor'
                  }
                  if (moduleId.includes('@shikijs')) {
                    return 'shiki'
                  }
                }
                return null
              },
            },
          ],
        },
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
})
