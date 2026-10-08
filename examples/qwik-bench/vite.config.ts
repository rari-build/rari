import { rariQwik } from '@rari/qwik/vite'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [rariQwik(), tailwindcss()],
})
