import path from 'node:path'
import { resolveAlias } from '@rari/vite/alias-resolver'
import { describe, expect, it } from 'vite-plus/test'

describe('alias-resolver', () => {
  describe('resolveAlias', () => {
    it('should resolve exact alias match', () => {
      const source = '@components'
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/components'))
    })

    it('should resolve alias with path', () => {
      const source = '@components/Button'
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/components/Button'))
    })

    it('should return null when no alias matches', () => {
      const source = '@utils/helper'
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(result).toBeNull()
    })

    it('should handle relative alias paths', () => {
      const source = '@lib'
      const aliases = {
        '@lib': 'src/lib',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(result).toBe(path.resolve('/project', 'src/lib'))
    })

    it('should handle alias with trailing slash', () => {
      const source = '@components/Button/index'
      const aliases = {
        '@components': '/src/components/',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/components/Button/index'))
    })

    it('should prefer longer alias matches', () => {
      const source = '@components/ui/Button'
      const aliases = {
        '@components': '/src/components',
        '@components/ui': '/src/ui',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/ui/Button'))
    })

    it('should handle empty aliases object', () => {
      const source = '@components/Button'
      const aliases = {}
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(result).toBeNull()
    })

    it('should handle absolute alias paths', () => {
      const source = '@shared'
      const aliases = {
        '@shared': '/absolute/path/to/shared',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/absolute/path/to/shared'))
    })

    it('should not match partial alias names', () => {
      const source = '@component'
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(result).toBeNull()
    })

    it('should handle Windows-style paths', () => {
      const source = '@components/Button'
      const aliases = {
        '@components': 'C:\\project\\src\\components',
      }
      const projectRoot = 'C:\\project'

      const result = resolveAlias(source, aliases, projectRoot)

      const aliasPath = 'C:\\project\\src\\components'
      const expected = path.isAbsolute(aliasPath)
        ? path.join(aliasPath, 'Button')
        : path.resolve(projectRoot, aliasPath, 'Button')

      expect(path.normalize(result!)).toBe(path.normalize(expected))
    })

    it('should handle nested alias paths', () => {
      const source = '@ui/components/Button/styles'
      const aliases = {
        '@ui': '/src/ui',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/ui/components/Button/styles'))
    })

    it('should handle alias with dots', () => {
      const source = '@lib.utils'
      const aliases = {
        '@lib.utils': '/src/lib/utils',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/lib/utils'))
    })

    it('should handle multiple aliases', () => {
      const aliases = {
        '@components': '/src/components',
        '@utils': '/src/utils',
        '@lib': '/src/lib',
      }
      const projectRoot = '/project'

      expect(path.normalize(resolveAlias('@components/Button', aliases, projectRoot)!)).toBe(path.normalize('/src/components/Button'))
      expect(path.normalize(resolveAlias('@utils/helper', aliases, projectRoot)!)).toBe(path.normalize('/src/utils/helper'))
      expect(path.normalize(resolveAlias('@lib/api', aliases, projectRoot)!)).toBe(path.normalize('/src/lib/api'))
    })

    it('should handle alias with special characters', () => {
      const source = '@my-components/Button'
      const aliases = {
        '@my-components': '/src/my-components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/my-components/Button'))
    })

    it('should resolve to absolute path when alias is relative', () => {
      const source = '@components'
      const aliases = {
        '@components': './src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      const expected = path.resolve(projectRoot, 'src', 'components')
      expect(result).not.toBeNull()
      expect(path.isAbsolute(result!)).toBe(true)
      expect(path.normalize(result!)).toBe(path.normalize(expected))
    })

    it('should handle empty string source', () => {
      const source = ''
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(result).toBeNull()
    })

    it('should handle URL-encoded characters in source', () => {
      const source = '@components/My%20Component'
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/components/My%20Component'))
    })

    it('should handle Unicode characters in source', () => {
      const source = '@components/µComponent'
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/components/µComponent'))
    })

    it('should handle emoji characters in source', () => {
      const source = '@components/Button🚀'
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/components/Button🚀'))
    })

    it('should handle very long alias keys', () => {
      const longAlias = `@${'a'.repeat(1000)}`
      const source = `${longAlias}/Component`
      const aliases = {
        [longAlias]: '/src/components',
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize('/src/components/Component'))
    })

    it('should handle very long paths', () => {
      const longPath = `/${'very/'.repeat(100)}deep/path`
      const source = '@components/Button'
      const aliases = {
        '@components': longPath,
      }
      const projectRoot = '/project'

      const result = resolveAlias(source, aliases, projectRoot)

      expect(path.normalize(result!)).toBe(path.normalize(`${longPath}/Button`))
    })

    it('should handle null source gracefully', () => {
      const source = null as any
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle undefined source gracefully', () => {
      const source = undefined as any
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle number as source', () => {
      const source = 123 as any
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle object as source', () => {
      const source = { path: '@components' } as any
      const aliases = {
        '@components': '/src/components',
      }
      const projectRoot = '/project'

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle null aliases gracefully', () => {
      const source = '@components/Button'
      const aliases = null as any
      const projectRoot = '/project'

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle undefined aliases gracefully', () => {
      const source = '@components/Button'
      const aliases = undefined as any
      const projectRoot = '/project'

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle array as aliases', () => {
      const source = '@components/Button'
      const aliases = ['@components', '/src/components'] as any
      const projectRoot = '/project'

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle null projectRoot gracefully', () => {
      const source = '@components/Button'
      const aliases = {
        '@components': 'src/components',
      }
      const projectRoot = null as any

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle undefined projectRoot gracefully', () => {
      const source = '@components/Button'
      const aliases = {
        '@components': 'src/components',
      }
      const projectRoot = undefined as any

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle number as projectRoot', () => {
      const source = '@components/Button'
      const aliases = {
        '@components': 'src/components',
      }
      const projectRoot = 123 as any

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle object as projectRoot', () => {
      const source = '@components/Button'
      const aliases = {
        '@components': 'src/components',
      }
      const projectRoot = { path: '/project' } as any

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle array as projectRoot', () => {
      const source = '@components/Button'
      const aliases = {
        '@components': 'src/components',
      }
      const projectRoot = ['/project'] as any

      expect(() => resolveAlias(source, aliases, projectRoot)).toThrow(TypeError)
    })

    it('should handle empty string as projectRoot', () => {
      const source = '@components/Button'
      const aliases = {
        '@components': 'src/components',
      }
      const projectRoot = ''

      const result = resolveAlias(source, aliases, projectRoot)

      expect(result).not.toBeNull()
      expect(result!.endsWith('src/components/Button')).toBe(true)
    })
  })
})
