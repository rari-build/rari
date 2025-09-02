import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react-oxc'
import { defineRariOptions, rari, rariRouter } from 'rari'
import { defineConfig } from 'rolldown-vite'

const rariOptions = defineRariOptions({
  caching: {
    routes: {
      '/about': 'public, max-age=1800, s-maxage=3600',
    },
  },
})

export default defineConfig({
  plugins: [rari(rariOptions), rariRouter(), react(), tailwindcss()],
})
