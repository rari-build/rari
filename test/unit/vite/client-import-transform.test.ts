import {
  locateExportDefaultValue,
  rewriteExportDefaultAsBinding,
  scanImportStatements,
} from '@rari/vite/analysis/directives'
import { buildClientReferenceReplacementFromImport } from '@rari/vite/transform/client-import'
import { describe, expect, it } from 'vite-plus/test'

describe('buildClientReferenceReplacementFromImport', () => {
  it('builds registerClientReference bindings for default and aliased named imports', () => {
    const [imp] = scanImportStatements(`import Button, { Card as TheCard } from './ui'\n`)
    const replacement = buildClientReferenceReplacementFromImport(imp, 'src/ui.tsx')

    expect(replacement.helpers).toEqual(['registerClientReference'])
    expect(replacement.code).toContain('"default"')
    expect(replacement.code).toContain('"Card"')
    expect(replacement.code).toContain('const TheCard = registerClientReference')
    expect(replacement.code).not.toContain('import {')
  })

  it('builds createClientModuleProxy for namespace imports', () => {
    const [imp] = scanImportStatements(`import * as ClientUI from "./ClientButton.tsx"\n`)
    const replacement = buildClientReferenceReplacementFromImport(
      imp,
      'src/components/ClientButton.tsx',
    )

    expect(replacement).toEqual({
      helpers: ['createClientModuleProxy'],
      code: `const ClientUI = createClientModuleProxy("src/components/ClientButton.tsx");`,
    })
  })

  it('emits both helpers when default and namespace are combined', () => {
    const [imp] = scanImportStatements(`import React, * as ReactNS from 'react'\n`)
    const replacement = buildClientReferenceReplacementFromImport(imp, 'react')

    expect(replacement.helpers).toContain('registerClientReference')
    expect(replacement.helpers).toContain('createClientModuleProxy')
    expect(replacement.code).toContain('const React = registerClientReference')
    expect(replacement.code).toContain('const ReactNS = createClientModuleProxy')
  })

  it('returns empty code for imports with only inline type-only specifiers', () => {
    const [imp] = scanImportStatements(`import { type Props } from './types'\n`)
    const replacement = buildClientReferenceReplacementFromImport(imp, 'src/types.ts')

    expect(replacement).toEqual({ code: '', helpers: [] })
  })
})

describe('rewriteExportDefaultAsBinding', () => {
  it('rewrites arrow defaults without truncating at inner semicolons', () => {
    const code = `'use server'\nexport default async () => {\n  doA();\n  doB()\n}\n`
    const rewritten = rewriteExportDefaultAsBinding(code, '__default_export__')

    expect(rewritten).toContain('const __default_export__ = async () => {\n  doA();\n  doB()\n}')
    expect(rewritten).toContain('export default __default_export__')
    expect(rewritten).not.toContain('export default async () => {\n  doA()')
  })

  it('leaves named function declarations alone', () => {
    const code = `'use server'\nexport default async function save() {\n  return 1\n}\n`

    expect(rewriteExportDefaultAsBinding(code, '__default_export__')).toBeNull()
    expect(locateExportDefaultValue(code)?.bindingName).toBe('save')
  })

  it('rewrites simple identifier defaults', () => {
    const code = `'use server'\nasync function save() {}\nexport default save\n`
    const rewritten = rewriteExportDefaultAsBinding(code, '__default_export__')

    expect(rewritten).toBe(
      `'use server'\nasync function save() {}\nconst __default_export__ = save;\nexport default __default_export__\n`,
    )
  })

  it('keeps scanning for next-line arrow bodies', () => {
    const code = `'use server'\nexport default () =>\n  value\n`
    const located = locateExportDefaultValue(code)
    const rewritten = rewriteExportDefaultAsBinding(code, '__default_export__')

    expect(code.slice(located!.valueStart, located!.valueEnd).trim()).toBe('() =>\n  value')
    expect(rewritten).toContain('const __default_export__ = () =>\n  value')
    expect(rewritten).toContain('export default __default_export__')
  })

  it('keeps scanning for operator-continued default exports', () => {
    const code = `'use server'\nexport default a\n  + b\n`
    const located = locateExportDefaultValue(code)
    const rewritten = rewriteExportDefaultAsBinding(code, '__default_export__')

    expect(code.slice(located!.valueStart, located!.valueEnd).trim()).toBe('a\n  + b')
    expect(rewritten).toContain('const __default_export__ = a\n  + b')
  })
})
