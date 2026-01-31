import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: ['src/index.ts', 'src/railway.ts', 'src/render.ts'],
  format: 'esm',
  dts: true,
  minify: true,
  noExternal: ['@rari/colors'],
})
