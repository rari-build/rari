import { describe, expect, it } from 'vite-plus/test'
import {
  buildNamespaceClientReferenceReplacement,
  NAMESPACE_IMPORT_LINE_REGEX,
} from '../../../packages/rari/src/vite/client-import-transform'

describe('namespace client import transform', () => {
  it('matches namespace import lines', () => {
    const match = 'import * as ClientUI from "./ClientButton.tsx"'.match(NAMESPACE_IMPORT_LINE_REGEX)

    expect(match?.[1]).toBe('ClientUI')
    expect(match?.[2]).toBe('./ClientButton.tsx')
  })

  it('does not match default or named imports', () => {
    expect('import Client from "./ClientButton.tsx"'.match(NAMESPACE_IMPORT_LINE_REGEX)).toBeNull()
    expect('import { Client } from "./ClientButton.tsx"'.match(NAMESPACE_IMPORT_LINE_REGEX)).toBeNull()
  })

  it('builds createClientModuleProxy replacement', () => {
    expect(buildNamespaceClientReferenceReplacement('ClientUI', 'src/components/ClientButton.tsx'))
      .toBe(`import { createClientModuleProxy } from "react-server-dom-rari/server";
const ClientUI = createClientModuleProxy("src/components/ClientButton.tsx");`)
  })
})
