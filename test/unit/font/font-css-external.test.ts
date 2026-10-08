import type { Plugin } from 'vite-plus'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { build, createLogger } from 'vite-plus'
import { describe, expect, it } from 'vite-plus/test'
import { createFontPlugin } from '../../../packages/rari/src/vite/font/plugin'

function createWarnCapturingLogger() {
  const logger = createLogger('warn')
  const messages: string[] = []
  logger.warnOnce = message => {
    messages.push(message)
  }
  return {
    logger,
    unresolvedCssUrlWarnings: () =>
      messages.filter(message => message.includes("didn't resolve at build time")),
  }
}

async function buildFontFixture(plugins: readonly Plugin[]): Promise<string[]> {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-font-css-'))
  fs.writeFileSync(
    path.join(dir, 'style.css'),
    `@font-face { src: url("/assets/Geist-abcd1234.woff2") format("woff2"); }\n`,
  )
  fs.writeFileSync(path.join(dir, 'main.js'), `import './style.css'\n`)

  const { logger, unresolvedCssUrlWarnings } = createWarnCapturingLogger()
  await build({
    root: dir,
    logLevel: 'warn',
    customLogger: logger,
    plugins: [...plugins],
    build: {
      write: false,
      rolldownOptions: {
        input: path.join(dir, 'main.js'),
      },
    },
  })

  return unresolvedCssUrlWarnings()
}

describe('font CSS public URL externalization', () => {
  it('warns without the font plugin when /assets/*.woff2 CSS urls are unresolved', async () => {
    const unresolved = await buildFontFixture([])
    expect(unresolved.some(message => message.includes('.woff2'))).toBe(true)
  })

  it('does not warn when the font plugin marks emitted font urls as external', async () => {
    const unresolved = await buildFontFixture([createFontPlugin()])
    expect(unresolved).toEqual([])
  })

  it('does not warn for localFont CSS across shared multi-env builds', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-font-css-'))
    fs.writeFileSync(path.join(dir, 'Geist.woff2'), Buffer.from('fake-font-bytes'))
    fs.writeFileSync(
      path.join(dir, 'fonts.ts'),
      `
import localFont from 'rari/font/local'
export const geist = localFont({
  src: './Geist.woff2',
  variable: '--font-geist',
  adjustFontFallback: false,
})
`,
    )
    fs.writeFileSync(
      path.join(dir, 'main.js'),
      `import { geist } from './fonts.ts'\nconsole.log(geist.className)\n`,
    )

    const { logger, unresolvedCssUrlWarnings } = createWarnCapturingLogger()
    await build({
      root: dir,
      logLevel: 'warn',
      customLogger: logger,
      plugins: [createFontPlugin()],
      builder: {
        sharedConfigBuild: false,
        sharedPlugins: true,
      },
      environments: {
        client: {
          build: {
            write: false,
            rolldownOptions: {
              input: path.join(dir, 'main.js'),
            },
          },
        },
        ssr: {
          build: {
            write: false,
            rolldownOptions: {
              input: path.join(dir, 'main.js'),
            },
          },
        },
      },
      build: {
        write: false,
        rolldownOptions: {
          input: path.join(dir, 'main.js'),
        },
      },
    })

    expect(unresolvedCssUrlWarnings()).toEqual([])
  })
})
