import type { Plugin } from 'vite-plus'
import {
  hasTopLevelUseClientDirective,
  hasTopLevelUseServerDirective,
  stripTopLevelDirective,
} from '../analysis/directives'

export function createStripReactDirectivesPlugin(): Plugin {
  return {
    name: 'rari:strip-react-directives',
    transform(code, id) {
      if (id.startsWith('\0') || id.includes('virtual:')) return null

      let next = code
      let changed = false

      if (hasTopLevelUseClientDirective(code)) {
        next = stripTopLevelDirective(next, 'use client')
        changed = true
      }

      if (hasTopLevelUseServerDirective(code)) {
        next = stripTopLevelDirective(next, 'use server')
        changed = true
      }

      return changed ? { code: next, map: null } : null
    },
  }
}
