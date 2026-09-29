import type { Plugin } from 'vite-plus'
import {
  hasTopLevelUseClientDirective,
  hasTopLevelUseServerDirective,
} from '../analysis/directives'

const USE_CLIENT_DIRECTIVE_REGEX = /^['"]use client['"];?\s*$/gm
const USE_SERVER_DIRECTIVE_REGEX = /^['"]use server['"];?\s*$/gm

export function createStripReactDirectivesPlugin(): Plugin {
  return {
    name: 'rari:strip-react-directives',
    transform(code, id) {
      if (id.startsWith('\0') || id.includes('virtual:')) return null

      let next = code
      let changed = false

      if (hasTopLevelUseClientDirective(code)) {
        next = next.replace(USE_CLIENT_DIRECTIVE_REGEX, '')
        changed = true
      }

      if (hasTopLevelUseServerDirective(code)) {
        next = next.replace(USE_SERVER_DIRECTIVE_REGEX, '')
        changed = true
      }

      return changed ? { code: next, map: null } : null
    },
  }
}
