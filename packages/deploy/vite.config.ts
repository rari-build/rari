import { defineConfig } from 'vite-plus'

export default defineConfig({
  pack: {
    entry: ['src/index.ts', 'src/railway.ts', 'src/render.ts'],
    minify: true,
  },
})
