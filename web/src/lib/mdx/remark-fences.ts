// oxlint-disable typescript/prefer-readonly-parameter-types
import { visit } from './visit'

interface ASTAttribute {
  type: string
  name: string
  value?: string
}

export interface ASTNode {
  type: string
  name?: string
  value?: string
  lang?: string | null
  meta?: string | null
  children?: ASTNode[]
  attributes?: ASTAttribute[]
  data?: unknown
}

interface Highlighter {
  readonly codeToHtml: (
    code: string,
    options: Readonly<{
      readonly lang: string
      readonly themes: { readonly light: string; readonly dark: string }
      readonly defaultColor: false
    }>,
  ) => string
}

export interface RemarkFencesOptions {
  readonly highlighter?: Highlighter
  readonly themes?: { readonly light: string; readonly dark: string }
}

const TITLE_META_REGEX = /title=["']([^"']+)["']/
const PACKAGE_MANAGER_LINE_REGEX = /^(pnpm|npm|yarn|bun):\s*(.*)/
const PACKAGE_MANAGERS = ['pnpm', 'npm', 'yarn', 'bun'] as const
const PRE_STYLE_REGEX = /<pre([^>]*) style="[^"]*"/g

type PackageManager = (typeof PACKAGE_MANAGERS)[number]

function attr(name: string, value: string): ASTAttribute {
  return { type: 'mdxJsxAttribute', name, value }
}

function jsxElement(name: string, attributes: ASTAttribute[], children: ASTNode[] = []): ASTNode {
  return {
    type: 'mdxJsxFlowElement',
    name,
    attributes,
    children,
    data: { _mdxExplicitJsx: true },
  }
}

function parseTitle(meta: string): string | undefined {
  if (meta === '') return undefined
  return TITLE_META_REGEX.exec(meta)?.[1]
}

function parsePackageManagerCommands(value: string): Partial<Record<PackageManager, string>> {
  const commands: Partial<Record<PackageManager, string>> = {}

  for (const line of value.split('\n')) {
    const trimmed = line.trim()
    if (trimmed === '') continue
    const match = PACKAGE_MANAGER_LINE_REGEX.exec(trimmed)
    if (match == null) continue
    const pm = PACKAGE_MANAGERS.find(candidate => candidate === match[1])
    if (pm == null) continue
    commands[pm] = match[2]
  }

  return commands
}

function packageManagersWidget(value: string): ASTNode | null {
  const commands = parsePackageManagerCommands(value)
  const attributes: ASTAttribute[] = []
  for (const pm of PACKAGE_MANAGERS) {
    const command = commands[pm]
    if (command != null && command !== '') attributes.push(attr(pm, command))
  }
  if (attributes.length === 0) return null
  return jsxElement('PackageManagerTabs', attributes)
}

function highlightCode(
  code: string,
  lang: string,
  highlighter: Highlighter,
  themes: { readonly light: string; readonly dark: string },
): string | undefined {
  try {
    const html = highlighter.codeToHtml(code.trim(), {
      lang,
      themes,
      defaultColor: false,
    })
    return html.replace(PRE_STYLE_REGEX, '<pre$1')
  } catch (err) {
    console.error('Failed to highlight code fence:', err)
    return undefined
  }
}

function codeBlockWidget(
  lang: string,
  meta: string,
  value: string,
  options: RemarkFencesOptions,
): ASTNode | null {
  if (lang === '' && meta === '' && value === '') return null

  const language = lang !== '' ? lang : 'typescript'
  const attributes: ASTAttribute[] = [attr('language', language)]
  const title = parseTitle(meta)
  if (title != null && title !== '') attributes.push(attr('filename', title))

  if (options.highlighter != null && options.themes != null) {
    const highlightedHtml = highlightCode(value, language, options.highlighter, options.themes)
    if (highlightedHtml != null && highlightedHtml !== '') {
      attributes.push(attr('highlightedHtml', highlightedHtml))
    }
  }

  return jsxElement('CodeBlock', attributes, [{ type: 'text', value }])
}

function codeToWidget(node: ASTNode, options: RemarkFencesOptions): ASTNode | null {
  const lang = node.lang ?? ''
  const value = node.value ?? ''
  const meta = node.meta ?? ''

  if (lang === 'terminal' || (lang === 'bash' && /\bterminal\b/.test(meta))) {
    return jsxElement('TerminalBlock', [attr('command', value.trim())])
  }

  if (lang === 'package-managers') return packageManagersWidget(value)
  if (lang === 'mermaid') return jsxElement('MermaidChart', [], [{ type: 'text', value }])
  return codeBlockWidget(lang, meta, value, options)
}

export function remarkFences(options: RemarkFencesOptions = {}) {
  return (tree: ASTNode) => {
    visit(tree, (node: ASTNode, index?: number, parent?: ASTNode) => {
      if (node.type !== 'code' || parent?.children == null || index == null) return

      const replacement = codeToWidget(node, options)
      if (replacement != null) parent.children[index] = replacement
    })
  }
}
