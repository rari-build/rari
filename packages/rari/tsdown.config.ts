import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: {
    'index': 'src/index.ts',
    'client': 'src/client.ts',
    'vite': 'src/vite.ts',
    'cli': 'src/cli.ts',
    'platform': 'src/platform.ts',
    'image/index': 'src/image/index.ts',
    'og/index': 'src/og/index.ts',
    'mdx': 'src/mdx.ts',
    'runtime/actions': 'src/runtime/actions.ts',
    'proxy/runtime-executor': 'src/proxy/runtime-executor.ts',
  },
  dts: true,
  format: 'esm',
  minify: true,
  external: ['lightningcss', 'postcss', 'rolldown-vite', '@oxc-project/types', 'rolldown', '@rolldown/pluginutils'],
  noExternal: ['@rari/colors', /^@rari\/deploy(?:\/.*)?$/],
})
