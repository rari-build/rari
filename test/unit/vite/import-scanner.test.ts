import { scanImportStatements } from '@rari/vite/analysis/directives'
import { describe, expect, it } from 'vite-plus/test'

describe('scanImportStatements', () => {
  it('parses a default import', () => {
    const code = `import Button from './Button'\nexport const x = 1\n`
    const [imp] = scanImportStatements(code)

    expect(imp.source).toBe('./Button')
    expect(imp.defaultBinding).toBe('Button')
    expect(imp.namespaceBinding).toBeNull()
    expect(imp.named).toEqual([])
    expect(code.slice(imp.start, imp.end)).toBe(`import Button from './Button'`)
  })

  it('parses named imports with aliases', () => {
    const code = `import { Button as B, Card } from "./ui";`
    const [imp] = scanImportStatements(code)

    expect(imp.named).toEqual([
      { imported: 'Button', local: 'B', typeOnly: false },
      { imported: 'Card', local: 'Card', typeOnly: false },
    ])
    expect(code.slice(imp.start, imp.end)).toBe(code)
  })

  it('parses combined default and named imports', () => {
    const code = `import Button, { Card, Chip as C } from './ui'\n`
    const [imp] = scanImportStatements(code)

    expect(imp.defaultBinding).toBe('Button')
    expect(imp.named).toEqual([
      { imported: 'Card', local: 'Card', typeOnly: false },
      { imported: 'Chip', local: 'C', typeOnly: false },
    ])
  })

  it('parses namespace imports, including combined with default', () => {
    const [ns, combined] = scanImportStatements(
      `import * as UI from './ui'\nimport React, * as ReactNS from 'react'\n`,
    )

    expect(ns.namespaceBinding).toBe('UI')
    expect(ns.defaultBinding).toBeNull()
    expect(combined.defaultBinding).toBe('React')
    expect(combined.namespaceBinding).toBe('ReactNS')
  })

  it('parses multi-line imports with a correct span', () => {
    const code = `import {\n  Button,\n  Card as TheCard,\n} from './ui'\nconst after = 1\n`
    const [imp] = scanImportStatements(code)

    expect(imp.named).toEqual([
      { imported: 'Button', local: 'Button', typeOnly: false },
      { imported: 'Card', local: 'TheCard', typeOnly: false },
    ])
    expect(code.slice(imp.end)).toBe(`\nconst after = 1\n`)
  })

  it('parses side-effect imports', () => {
    const [imp] = scanImportStatements(`import './styles.css';\n`)

    expect(imp.sideEffectOnly).toBe(true)
    expect(imp.source).toBe('./styles.css')
  })

  it('marks type-only imports and inline type specifiers', () => {
    const code = `import type { Props } from './types'\nimport { type Meta, Button } from './ui'\n`
    const [typeImport, mixed] = scanImportStatements(code)

    expect(typeImport.typeOnly).toBe(true)
    expect(mixed.typeOnly).toBe(false)
    expect(mixed.named).toEqual([
      { imported: 'Meta', local: 'Meta', typeOnly: true },
      { imported: 'Button', local: 'Button', typeOnly: false },
    ])
  })

  it('treats `type` as a default binding when followed by from', () => {
    const [imp] = scanImportStatements(`import type from './type-module'\n`)

    expect(imp.typeOnly).toBe(false)
    expect(imp.defaultBinding).toBe('type')
  })

  it('parses string import names with aliases', () => {
    const [imp] = scanImportStatements(`import { "dash-name" as dashName } from './x'\n`)

    expect(imp.named).toEqual([{ imported: 'dash-name', local: 'dashName', typeOnly: false }])
  })

  it('ignores dynamic imports and import.meta', () => {
    const code = `const mod = await import('./lazy')\nconsole.log(import.meta.url)\n`

    expect(scanImportStatements(code)).toEqual([])
  })

  it('ignores imports inside strings, template literals, and comments', () => {
    const code = [
      `const a = "import Fake from './fake'"`,
      'const b = `import Fake2 from "./fake2"`',
      `// import Fake3 from './fake3'`,
      `/* import Fake4 from './fake4' */`,
      `import Real from './real'`,
    ].join('\n')

    const imports = scanImportStatements(code)
    expect(imports).toHaveLength(1)
    expect(imports[0].defaultBinding).toBe('Real')
  })

  it('ignores imports inside JSX text', () => {
    const code = `import Real from './real'\nexport default function P() {\n  return <div>import Fake from './fake'</div>\n}\n`

    const imports = scanImportStatements(code)
    expect(imports).toHaveLength(1)
    expect(imports[0].source).toBe('./real')
  })

  it('does not treat identifiers containing "import" as imports', () => {
    const code = `const reimport = 1\nconst importantThing = reimport\n`

    expect(scanImportStatements(code)).toEqual([])
  })

  it('includes the trailing semicolon in the span', () => {
    const code = `import A from './a';  \nconst x = 1\n`
    const [imp] = scanImportStatements(code)

    expect(code.slice(imp.start, imp.end)).toBe(`import A from './a';`)
  })

  it('scans multiple statements with correct ordering', () => {
    const code = `import A from './a'\nimport { B } from './b'\nimport * as C from './c'\n`
    const imports = scanImportStatements(code)

    expect(imports.map(imp => imp.source)).toEqual(['./a', './b', './c'])
  })

  it('recovers after malformed import-like code', () => {
    const code = `import !!!\nimport Real from './real'\n`
    const imports = scanImportStatements(code)

    expect(imports).toHaveLength(1)
    expect(imports[0].defaultBinding).toBe('Real')
  })
})
