import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { contentHash } from '@rari/shared/utils/content-hash'
import { getComponentId, getReadableComponentId } from '@rari/vite/analysis/component-ids'
import {
  getDirectives,
  hasTopLevelUseClientDirective,
  hasTopLevelUseServerDirective,
} from '@rari/vite/analysis/directives'
import { describe, expect, it } from 'vite-plus/test'
import { castMock } from '../../helpers/mock-cast'

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
    const fixture = castMock<{ cases: ComponentIdCase[] }>(
      JSON.parse(fs.readFileSync(path.join(fixturesDir, 'component-ids.json'), 'utf8')),
    )
    const projectRoot = path.join(os.tmpdir(), 'rari-analysis-golden')

    for (const testCase of fixture.cases) {
      expect(getReadableComponentId(testCase.input)).toBe(testCase.readable)
      expect(contentHash(testCase.input)).toBe(testCase.id.split('_').pop())
      expect(getComponentId(path.join(projectRoot, testCase.input), projectRoot)).toBe(testCase.id)
    }
  })

  it('matches directive fixtures', () => {
    const fixture = castMock<{ cases: DirectiveCase[] }>(
      JSON.parse(fs.readFileSync(path.join(fixturesDir, 'directives.json'), 'utf8')),
    )

    for (const testCase of fixture.cases) {
      const directives = getDirectives(testCase.source)
      expect({
        id: testCase.id,
        hasUseClient: directives.hasUseClient,
        hasUseServer: directives.hasUseServer,
        topLevelUseClient: hasTopLevelUseClientDirective(testCase.source),
        topLevelUseServer: hasTopLevelUseServerDirective(testCase.source),
      }).toEqual({
        id: testCase.id,
        hasUseClient: testCase.hasUseClient,
        hasUseServer: testCase.hasUseServer,
        topLevelUseClient: testCase.topLevelUseClient,
        topLevelUseServer: testCase.topLevelUseServer,
      })
    }
  })
})
