import { rari, rariRouter } from 'rari/server'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    ...rari(),
    rariRouter({
      appDir: 'src/app',
      useAppRouter: true,
      extensions: ['.tsx', '.jsx', '.ts', '.js'],
    }),
  ],
  server: {
    port: 3001,
  },
})
