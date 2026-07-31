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

    expect(replacement).toContain(
      'import { registerClientReference } from "react-server-dom-rari/server";',
    )
    expect(replacement).toContain('"default"')
    expect(replacement).toContain('"Card"')
    expect(replacement).toContain('const TheCard = registerClientReference')
  })

  it('builds createClientModuleProxy for namespace imports', () => {
    const [imp] = scanImportStatements(`import * as ClientUI from "./ClientButton.tsx"\n`)
    const replacement = buildClientReferenceReplacementFromImport(
      imp,
      'src/components/ClientButton.tsx',
    )

    expect(replacement).toBe(
      `import { createClientModuleProxy } from "react-server-dom-rari/server";\nconst ClientUI = createClientModuleProxy("src/components/ClientButton.tsx");`,
    )
  })

  it('emits both helpers when default and namespace are combined', () => {
    const [imp] = scanImportStatements(`import React, * as ReactNS from 'react'\n`)
    const replacement = buildClientReferenceReplacementFromImport(imp, 'react')

    expect(replacement).toContain('registerClientReference')
    expect(replacement).toContain('createClientModuleProxy')
    expect(replacement).toContain('const React = registerClientReference')
    expect(replacement).toContain('const ReactNS = createClientModuleProxy')
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
})
