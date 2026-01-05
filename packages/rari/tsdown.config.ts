import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: {
    'index': 'src/index.ts',
    'client': 'src/client.ts',
    'vite': 'src/vite.ts',
    'cli': 'src/cli.ts',
    'mdx': 'src/mdx.ts',
    'runtime/actions': 'src/runtime/actions.ts',
  },
  dts: true,
  fixedExtension: true,
  format: 'esm',
})
