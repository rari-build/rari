import type { HighlighterCore } from '@shikijs/core'
import type { ThemeRegistration } from '@shikijs/types'
import { createHighlighterCore } from '@shikijs/core'
import { createOnigurumaEngine } from '@shikijs/engine-oniguruma'

let highlighter: HighlighterCore | null = null

function replaceThemeColors(
  theme: ThemeRegistration,
  replacements: Record<string, string>,
): ThemeRegistration {
  let themeString = JSON.stringify(theme)
  for (const [oldColor, newColor] of Object.entries(replacements)) {
    themeString = themeString.replaceAll(oldColor, newColor)
    themeString = themeString.replaceAll(oldColor.toLowerCase(), newColor)
    themeString = themeString.replaceAll(oldColor.toUpperCase(), newColor)
  }

  return JSON.parse(themeString)
}

export const SHIKI_THEME = 'github-dark'

export async function getHighlighter(): Promise<HighlighterCore> {
  if (!highlighter) {
    highlighter = await createHighlighterCore({
      themes: [
        import('@shikijs/themes/github-dark').then(t =>
          replaceThemeColors(t.default ?? t, {
            '#6A737D': '#8B949E', // comments: boost contrast on dark bg
          }),
        ),
      ],
      langs: [
        import('@shikijs/langs/bash'),
        import('@shikijs/langs/tsx'),
        import('@shikijs/langs/typescript'),
      ],
      engine: createOnigurumaEngine(import('@shikijs/engine-oniguruma/wasm-inlined')),
    })
  }

  return highlighter
}
