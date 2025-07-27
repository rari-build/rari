import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    client: 'src/client.ts',
    server: 'src/server.ts',
    cli: 'src/cli.ts',
  },
  external: ['fsevents'],
  dts: true,
})
