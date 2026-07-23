import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  getComponentId,
  getReadableComponentId,
  hashString,
} from '@rari/vite/analysis/component-ids'
import {
  getDirectives,
  hasTopLevelUseClientDirective,
  hasTopLevelUseServerDirective,
} from '@rari/vite/analysis/directives'
import { describe, expect, it } from 'vite-plus/test'

const fixturesDir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  '../../fixtures/analysis',
)

interface ComponentIdCase {
  input: string
  readable: string
  id: string
}

interface DirectiveCase {
  id: string
  source: string
  hasUseClient: boolean
  hasUseServer: boolean
  topLevelUseClient: boolean
  topLevelUseServer: boolean
}

describe('analysis goldens (shared with Rust)', () => {
  it('matches component ID fixtures', () => {
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- golden fixture JSON
    const fixture = JSON.parse(
      fs.readFileSync(path.join(fixturesDir, 'component-ids.json'), 'utf8'),
    ) as { cases: ComponentIdCase[] }
    const projectRoot = path.join(os.tmpdir(), 'rari-analysis-golden')

    for (const testCase of fixture.cases) {
      expect(getReadableComponentId(testCase.input)).toBe(testCase.readable)
      expect(hashString(testCase.input)).toBe(testCase.id.split('_').pop())
      expect(getComponentId(path.join(projectRoot, testCase.input), projectRoot)).toBe(testCase.id)
    }
  })

  it('matches directive fixtures', () => {
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- golden fixture JSON
    const fixture = JSON.parse(
      fs.readFileSync(path.join(fixturesDir, 'directives.json'), 'utf8'),
    ) as { cases: DirectiveCase[] }

    for (const testCase of fixture.cases) {
      const directives = getDirectives(testCase.source)
      expect(directives.hasUseClient, testCase.id).toBe(testCase.hasUseClient)
      expect(directives.hasUseServer, testCase.id).toBe(testCase.hasUseServer)
      expect(hasTopLevelUseClientDirective(testCase.source), testCase.id).toBe(
        testCase.topLevelUseClient,
      )
      expect(hasTopLevelUseServerDirective(testCase.source), testCase.id).toBe(
        testCase.topLevelUseServer,
      )
    }
  })
})
