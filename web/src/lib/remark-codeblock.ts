import { visit } from 'unist-util-visit'

const BACKTICK_REGEX = /`([\s\S]*)`/
const ESCAPED_BACKTICK_REGEX = /\\\\\\`/g
const SINGLE_BACKTICK_REGEX = /\\`/g
const ESCAPED_DOLLAR_REGEX = /\\\$/g
const PRE_STYLE_REGEX = /<pre([^>]*) style="[^"]*"/g

interface Position {
  start: { offset: number, line: number, column: number }
  end: { offset: number, line: number, column: number }
}

interface ASTNode {
  type: string
  name?: string
  value?: string
  children?: ASTNode[]
  attributes?: any[]
  position?: Position
  data?: any
}

interface VFile {
  value: string | Uint8Array
}

interface Highlighter {
  codeToHtml: (code: string, options: { lang: string, theme: string }) => string
}

export function remarkCodeBlock(options: { highlighter: Highlighter, theme: string }) {
  const { highlighter, theme } = options

  return (tree: ASTNode, file: VFile) => {
    visit(tree, (node: ASTNode) => {
      if (node.type !== 'mdxJsxFlowElement' && node.type !== 'mdxJsxTextElement')
        return
      if (node.name !== 'CodeBlock')
        return

      let code = ''

      function extractText(children: ASTNode[]): string {
        let text = ''
        for (const child of children) {
          if (child.type === 'text') {
            text += child.value
          }
          else if (child.type === 'mdxFlowExpression' || child.type === 'mdxTextExpression') {
            if (child.value && child.position && file.value) {
              const sourceText = String(file.value)
              const start = child.position.start.offset
              const end = child.position.end.offset
              const originalText = sourceText.substring(start, end)

              const match = originalText.match(BACKTICK_REGEX)
              if (match) {
                let extracted = match[1]

                extracted = extracted.replace(ESCAPED_BACKTICK_REGEX, '\u02CB')
                extracted = extracted.replace(SINGLE_BACKTICK_REGEX, '`')
                extracted = extracted.replace(ESCAPED_DOLLAR_REGEX, '$')

                text += extracted
              }
            }
          }
          else if (child.children) {
            text += extractText(child.children)
          }
        }

        return text
      }

      if (node.children && node.children.length > 0)
        code = extractText(node.children)
      if (!code.trim())
        return

      const languageProp = node.attributes?.find(
        (attr: { type: string, name: string, value?: string }) => attr.type === 'mdxJsxAttribute' && attr.name === 'language',
      )
      const language = languageProp?.value || 'typescript'

      try {
        let highlightedHtml = highlighter.codeToHtml(code.trim(), {
          lang: language,
          theme,
        })
        highlightedHtml = highlightedHtml.replace(PRE_STYLE_REGEX, '<pre$1')
        if (!node.attributes)
          node.attributes = []
        node.attributes.push({
          type: 'mdxJsxAttribute',
          name: 'highlightedHtml',
          value: highlightedHtml,
        })
      }
      catch (err) {
        console.error('Failed to highlight code in CodeBlock:', err)
      }
    })
  }
}
