import type { HighlighterCore } from '@shikijs/core'
import type { ThemeRegistration } from '@shikijs/types'
import { createHighlighterCore } from '@shikijs/core'
import { createOnigurumaEngine } from '@shikijs/engine-oniguruma'

let highlighter: HighlighterCore | null = null

function isThemeRegistration(value: unknown): value is ThemeRegistration {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof Reflect.get(value, 'name') === 'string' &&
    typeof Reflect.get(value, 'type') === 'string'
  )
}

function replaceThemeColors(
  theme: ThemeRegistration,
  replacements: Readonly<Record<string, string>>,
): ThemeRegistration {
  let themeString = JSON.stringify(theme)
  for (const [oldColor, newColor] of Object.entries(replacements)) {
    themeString = themeString.replaceAll(oldColor, newColor)
    themeString = themeString.replaceAll(oldColor.toLowerCase(), newColor)
    themeString = themeString.replaceAll(oldColor.toUpperCase(), newColor)
  }

  const parsed: unknown = JSON.parse(themeString)
  if (!isThemeRegistration(parsed)) return theme

  return parsed
}

export const SHIKI_THEMES = {
  light: 'github-light-high-contrast',
  dark: 'github-dark',
} as const

export async function getHighlighter(): Promise<HighlighterCore> {
  highlighter ??= await createHighlighterCore({
    themes: [
      import('@shikijs/themes/github-dark').then(t =>
        replaceThemeColors(t.default, {
          '#6A737D': '#8B949E', // comments: boost contrast on dark bg
        }),
      ),
      import('@shikijs/themes/github-light-high-contrast').then(t =>
        replaceThemeColors(t.default, {
          '#6A737D': '#424A53',
          '#57606A': '#424A53',
        }),
      ),
    ],
    langs: [
      import('@shikijs/langs/bash'),
      import('@shikijs/langs/dockerfile'),
      import('@shikijs/langs/toml'),
      import('@shikijs/langs/tsx'),
      import('@shikijs/langs/typescript'),
      import('@shikijs/langs/yaml'),
    ],
    engine: createOnigurumaEngine(import('@shikijs/engine-oniguruma/wasm-inlined')),
  })

  return highlighter
}
