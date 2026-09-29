import type { Plugin } from 'vite-plus'
import { createStripReactDirectivesPlugin } from '@rari/vite/build/strip-react-directives'
import { describe, expect, it } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

type TransformResult = Readonly<{ code: string; map: null }> | null | undefined

async function callTransform(plugin: Plugin, code: string, id: string): Promise<TransformResult> {
  const hook = plugin.transform
  if (hook == null) throw new Error('expected transform hook')

  if (typeof hook === 'function') {
    return castMock(await hook.call(castMock({}), code, id))
  }
  if (typeof hook.handler === 'function') {
    return castMock(await hook.handler.call(castMock({}), code, id))
  }
  throw new Error('expected transform function')
}

describe('strip-react-directives', () => {
  it('strips use client and use server in transform', async () => {
    const plugin = createStripReactDirectivesPlugin()

    expect(
      await callTransform(
        plugin,
        `'use client'\nexport function Button() { return null }\n`,
        '/app/Button.tsx',
      ),
    ).toEqual({
      code: `export function Button() { return null }\n`,
      map: null,
    })

    expect(
      await callTransform(
        plugin,
        `'use server'\nexport async function save() { return 1 }\n`,
        '/app/actions.ts',
      ),
    ).toEqual({
      code: `export async function save() { return 1 }\n`,
      map: null,
    })
  })

  it('leaves modules without directives unchanged', async () => {
    const plugin = createStripReactDirectivesPlugin()
    expect(await callTransform(plugin, 'export const x = 1\n', '/app/util.ts')).toBeNull()
  })

  it('does not strip directive text inside template literals', async () => {
    const plugin = createStripReactDirectivesPlugin()
    const code = `'use client'\nexport const hint = \`\n'use client'\n\`\n`

    expect(await callTransform(plugin, code, '/app/Hint.tsx')).toEqual({
      code: `export const hint = \`\n'use client'\n\`\n`,
      map: null,
    })
  })

  it('preserves preceding comments when stripping the directive', async () => {
    const plugin = createStripReactDirectivesPlugin()
    const code = `/** @jsxImportSource react */\n'use client'\nexport function Button() { return null }\n`

    expect(await callTransform(plugin, code, '/app/Button.tsx')).toEqual({
      code: `/** @jsxImportSource react */\nexport function Button() { return null }\n`,
      map: null,
    })
  })

  it('does not strip directive-looking strings in expressions', async () => {
    const plugin = createStripReactDirectivesPlugin()
    const code = `'use client'\nexport const flag = 'use client' + suffix\n`

    expect(await callTransform(plugin, code, '/app/flag.ts')).toEqual({
      code: `export const flag = 'use client' + suffix\n`,
      map: null,
    })
  })

  it('leaves expression-only use client strings alone when not a directive', async () => {
    const plugin = createStripReactDirectivesPlugin()
    const code = `export const flag = 'use client' + suffix\n`
    expect(await callTransform(plugin, code, '/app/flag.ts')).toBeNull()
  })

  it('strips a standalone directive at EOF without trailing newline', async () => {
    const plugin = createStripReactDirectivesPlugin()
    expect(await callTransform(plugin, `'use client'`, '/app/Empty.tsx')).toEqual({
      code: '',
      map: null,
    })
    expect(await callTransform(plugin, `'use server'`, '/app/action.ts')).toEqual({
      code: '',
      map: null,
    })
  })
})
