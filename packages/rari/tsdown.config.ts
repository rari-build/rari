import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: {
    'index': 'src/index.ts',
    'client': 'src/client.ts',
    'vite': 'src/vite.ts',
    'cli': 'src/cli.ts',
    'runtime/actions': 'src/runtime/actions.ts',
    'hooks/useActionState': 'src/hooks/useActionState.ts',
  },
  external: ['fsevents'],
  dts: true,
  fixedExtension: true,
  format: 'esm',
})
