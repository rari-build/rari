import type { HighlighterCore } from '@shikijs/core'
import { createHighlighterCore } from '@shikijs/core'
import { createOnigurumaEngine } from '@shikijs/engine-oniguruma'
import bash from '@shikijs/langs/bash'
import tsx from '@shikijs/langs/tsx'
import typescript from '@shikijs/langs/typescript'
import githubDark from '@shikijs/themes/github-dark'

let shikiHighlighter: HighlighterCore | null = null

export async function getHighlighter(): Promise<HighlighterCore | null> {
  if (!shikiHighlighter) {
    try {
      shikiHighlighter = await createHighlighterCore({
        themes: [githubDark],
        langs: [typescript, tsx, bash],
        engine: createOnigurumaEngine(import('@shikijs/engine-oniguruma/wasm-inlined')),
      })
    }
    catch (error) {
      console.error('Failed to initialize syntax highlighter:', error)
      return null
    }
  }
  return shikiHighlighter
}

export const SHIKI_THEME = 'github-dark'
