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
})
