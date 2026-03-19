import fs from 'node:fs'
import path from 'node:path'
import { resolveIndexFile, resolveWithExtensions } from '@rari/vite/file-resolver'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

vi.mock('node:fs')

describe('file-resolver', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('resolveWithExtensions', () => {
    it('should return path with matching extension', () => {
      const basePath = '/test/component'
      const extensions = ['.tsx', '.ts', '.jsx', '.js']

      vi.mocked(fs.existsSync)
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(true)

      const result = resolveWithExtensions(basePath, extensions)

      expect(result).toBe(path.join('/test', 'component.ts'))
    })

    it('should return null when no extension matches', () => {
      const basePath = '/test/missing'
      const extensions = ['.tsx', '.ts']

      vi.mocked(fs.existsSync).mockReturnValue(false)

      const result = resolveWithExtensions(basePath, extensions)

      expect(result).toBeNull()
    })

    it('should return null when path does not exist', () => {
      const basePath = '/test/directory'
      const extensions = ['.tsx']

      vi.mocked(fs.existsSync).mockReturnValue(false)

      const result = resolveWithExtensions(basePath, extensions)

      expect(result).toBeNull()
    })

    it('should try extensions in order', () => {
      const basePath = '/test/component'
      const extensions = ['.tsx', '.ts', '.jsx', '.js']

      vi.mocked(fs.existsSync)
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(true)

      const result = resolveWithExtensions(basePath, extensions)

      expect(result).toBe(path.join('/test', 'component.js'))
      expect(fs.existsSync).toHaveBeenCalledTimes(4)
    })

    it('should handle empty extensions array', () => {
      const basePath = '/test/component'
      const extensions: string[] = []

      const result = resolveWithExtensions(basePath, extensions)

      expect(result).toBeNull()
    })

    it('should handle paths with existing extension', () => {
      const basePath = '/test/component.tsx'
      const extensions = ['.tsx', '.ts']

      vi.mocked(fs.existsSync).mockReturnValueOnce(true)

      const result = resolveWithExtensions(basePath, extensions)

      expect(result).toBe(path.join('/test', 'component.tsx'))
    })
  })

  describe('resolveIndexFile', () => {
    it('should resolve index file with matching extension', () => {
      const dirPath = '/test/components'
      const extensions = ['.tsx', '.ts', '.jsx', '.js']

      vi.mocked(fs.existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(true)

      vi.mocked(fs.statSync).mockReturnValue({
        isDirectory: () => true,
      } as any)

      const result = resolveIndexFile(dirPath, extensions)

      expect(result).toBe(path.join('/test', 'components', 'index.ts'))
    })

    it('should return null when no index file found', () => {
      const dirPath = '/test/empty'
      const extensions = ['.tsx', '.ts']

      vi.mocked(fs.existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(false)

      vi.mocked(fs.statSync).mockReturnValue({
        isDirectory: () => true,
      } as any)

      const result = resolveIndexFile(dirPath, extensions)

      expect(result).toBeNull()
    })

    it('should try extensions in order for index files', () => {
      const dirPath = '/test/components'
      const extensions = ['.tsx', '.ts', '.jsx', '.js']

      vi.mocked(fs.existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(true)

      vi.mocked(fs.statSync).mockReturnValue({
        isDirectory: () => true,
      } as any)

      const result = resolveIndexFile(dirPath, extensions)

      expect(result).toBe(path.join('/test', 'components', 'index.js'))
      expect(fs.existsSync).toHaveBeenCalledTimes(5)
    })

    it('should handle empty extensions array', () => {
      const dirPath = '/test/components'
      const extensions: string[] = []

      vi.mocked(fs.existsSync).mockReturnValueOnce(true)
      vi.mocked(fs.statSync).mockReturnValue({
        isDirectory: () => true,
      } as any)

      const result = resolveIndexFile(dirPath, extensions)

      expect(result).toBeNull()
      expect(fs.existsSync).toHaveBeenCalledWith(dirPath)
    })

    it('should handle paths with trailing slash', () => {
      const dirPath = '/test/components/'
      const extensions = ['.tsx']

      vi.mocked(fs.existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)

      vi.mocked(fs.statSync).mockReturnValue({
        isDirectory: () => true,
      } as any)

      const result = resolveIndexFile(dirPath, extensions)

      expect(result).toBe(path.join('/test', 'components', 'index.tsx'))
    })

    it('should prefer tsx over other extensions', () => {
      const dirPath = '/test/components'
      const extensions = ['.tsx', '.ts', '.jsx', '.js']

      vi.mocked(fs.existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)

      vi.mocked(fs.statSync).mockReturnValue({
        isDirectory: () => true,
      } as any)

      const result = resolveIndexFile(dirPath, extensions)

      expect(result).toBe(path.join('/test', 'components', 'index.tsx'))
      expect(fs.existsSync).toHaveBeenCalledTimes(2)
    })

    it('should return null when path exists but is not a directory', () => {
      const dirPath = '/test/file.txt'
      const extensions = ['.tsx', '.ts']

      vi.mocked(fs.existsSync).mockReturnValueOnce(true)
      vi.mocked(fs.statSync).mockReturnValue({
        isDirectory: () => false,
      } as any)

      const result = resolveIndexFile(dirPath, extensions)

      expect(result).toBeNull()
    })
  })

  describe('integration scenarios', () => {
    it('should resolve component with extension first, then index', () => {
      const basePath = '/test/Button'
      const extensions = ['.tsx', '.ts']

      vi.mocked(fs.existsSync).mockReturnValue(false)
      const withExt = resolveWithExtensions(basePath, extensions)
      expect(withExt).toBeNull()

      vi.mocked(fs.existsSync)
        .mockReturnValueOnce(true)
        .mockReturnValueOnce(true)

      vi.mocked(fs.statSync).mockReturnValue({
        isDirectory: () => true,
      } as any)

      const indexFile = resolveIndexFile(basePath, extensions)
      expect(indexFile).toBe(path.join('/test', 'Button', 'index.tsx'))
    })

    it('should handle Windows-style paths', () => {
      const basePath = 'C:\\test\\component'
      const extensions = ['.tsx']

      vi.mocked(fs.existsSync).mockReturnValue(true)

      const result = resolveWithExtensions(basePath, extensions)

      expect(result).toBe('C:\\test\\component.tsx')
    })

    it('should handle relative paths', () => {
      const basePath = './components/Button'
      const extensions = ['.tsx']

      vi.mocked(fs.existsSync).mockReturnValue(true)

      const result = resolveWithExtensions(basePath, extensions)

      expect(result).toBe('./components/Button.tsx')
    })
  })
})
