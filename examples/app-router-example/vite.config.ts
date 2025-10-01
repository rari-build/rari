import { rari, rariRouter } from 'rari/server'
import { defineConfig } from 'rolldown-vite'

export default defineConfig({
  plugins: [rari(), rariRouter()],
})
