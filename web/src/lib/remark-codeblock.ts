import { visit } from './visit'

const BACKTICK_REGEX = /`([\s\S]*)`/
const ESCAPED_BACKTICK_REGEX = /\\\\\\`/g
const SINGLE_BACKTICK_REGEX = /\\`/g
const ESCAPED_DOLLAR_REGEX = /\\\$/g
const PRE_STYLE_REGEX = /<pre([^>]*) style="[^"]*"/g

interface Position {
  start: { offset: number; line: number; column: number }
  end: { offset: number; line: number; column: number }
}

interface ASTAttribute {
  type: string
  name: string
  value?: string
}

interface ASTNode {
  type: string
  name?: string
  value?: string
  children?: ASTNode[]
  attributes?: ASTAttribute[]
  position?: Position
  data?: unknown
}

interface VFile {
  readonly value: string | Uint8Array
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

export function remarkCodeBlock(
  options: Readonly<{
    readonly highlighter: Highlighter
    readonly themes: { readonly light: string; readonly dark: string }
  }>,
) {
  const { highlighter, themes } = options

  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- tree is mutated in place (attributes are pushed onto matched nodes)
  return (tree: ASTNode, file: VFile) => {
    // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- node is mutated in place
    visit(tree, (node: ASTNode) => {
      if (node.type !== 'mdxJsxFlowElement' && node.type !== 'mdxJsxTextElement') return
      if (node.name !== 'CodeBlock') return

      let code = ''

      // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- ASTNode is a mutable AST type
      function extractText(children: readonly ASTNode[]): string {
        let text = ''
        for (const child of children) {
          if (child.type === 'text') {
            text += child.value
          } else if (child.type === 'mdxFlowExpression' || child.type === 'mdxTextExpression') {
            if (child.value != null && child.value !== '' && child.position != null) {
              const sourceText = String(file.value)
              const start = child.position.start.offset
              const end = child.position.end.offset
              const originalText = sourceText.substring(start, end)

              const match = BACKTICK_REGEX.exec(originalText)
              if (match) {
                let extracted = match[1]

                extracted = extracted.replace(ESCAPED_BACKTICK_REGEX, '\u02CB')
                extracted = extracted.replace(SINGLE_BACKTICK_REGEX, '`')
                extracted = extracted.replace(ESCAPED_DOLLAR_REGEX, '$')

                text += extracted
              }
            }
          } else if (child.children) {
            text += extractText(child.children)
          }
        }

        return text
      }

      if (node.children && node.children.length > 0) code = extractText(node.children)
      if (!code.trim()) return

      const languageProp = node.attributes?.find(
        attr => attr.type === 'mdxJsxAttribute' && attr.name === 'language',
      )
      const language =
        languageProp?.value != null && languageProp.value !== '' ? languageProp.value : 'typescript'

      try {
        let highlightedHtml = highlighter.codeToHtml(code.trim(), {
          lang: language,
          themes,
          defaultColor: false,
        })
        highlightedHtml = highlightedHtml.replace(PRE_STYLE_REGEX, '<pre$1')
        node.attributes ??= []
        node.attributes.push({
          type: 'mdxJsxAttribute',
          name: 'highlightedHtml',
          value: highlightedHtml,
        })
      } catch (err) {
        console.error('Failed to highlight code in CodeBlock:', err)
      }
    })
  }
}
