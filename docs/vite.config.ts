import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { rari, rariRouter } from 'rari/vite'
import { defineConfig } from 'rolldown-vite'

export default defineConfig({
  plugins: [rari(), rariRouter(), react(), tailwindcss()],
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
                }
                return null
              },
            },
          ],
        },
      },
    },
  },
})
