import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react-oxc'
import { rari, rariRouter } from 'rari'
import { defineConfig } from 'rolldown-vite'

export default defineConfig({
  plugins: [
    rari({
      preconnectDomains: [
        'www.googletagmanager.com',
        'www.google-analytics.com',
      ],
    }),
    rariRouter(),
    react(),
    tailwindcss(),
  ],
})
