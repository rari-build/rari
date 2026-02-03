import nodePath from 'node:path'
import process from 'node:process'
import { getBinaryPath, getInstallationInstructions } from '@rari/platform'
import { afterEach, describe, expect, it, vi } from 'vitest'

describe('platform', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('getInstallationInstructions', () => {
    it('should return installation instructions for darwin-arm64', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('darwin')
      vi.spyOn(process, 'arch', 'get').mockReturnValue('arm64')

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-darwin-arm64')
      expect(instructions).toContain('npm install')
      expect(instructions).toContain('pnpm add')
      expect(instructions).toContain('yarn add')
      expect(instructions).toContain('cargo install')
    })

    it('should return installation instructions for darwin-x64', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('darwin')
      vi.spyOn(process, 'arch', 'get').mockReturnValue('x64')

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-darwin-x64')
    })

    it('should return installation instructions for linux-x64', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('linux')
      vi.spyOn(process, 'arch', 'get').mockReturnValue('x64')

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-linux-x64')
    })

    it('should return installation instructions for linux-arm64', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('linux')
      vi.spyOn(process, 'arch', 'get').mockReturnValue('arm64')

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-linux-arm64')
    })

    it('should return installation instructions for win32-x64', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('win32')
      vi.spyOn(process, 'arch', 'get').mockReturnValue('x64')

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-win32-x64')
    })

    it('should throw error for unsupported platform', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('freebsd' as any)
      vi.spyOn(process, 'arch', 'get').mockReturnValue('x64')

      expect(() => getInstallationInstructions()).toThrow(
        'Unsupported platform: freebsd',
      )
    })

    it('should throw error for unsupported architecture', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('darwin')
      vi.spyOn(process, 'arch', 'get').mockReturnValue('ia32' as any)

      expect(() => getInstallationInstructions()).toThrow(
        'Unsupported architecture: ia32',
      )
    })
  })

  describe('platform detection', () => {
    it('should handle all supported platform combinations', () => {
      const platforms = [
        { platform: 'darwin', arch: 'x64', expected: 'rari-darwin-x64' },
        { platform: 'darwin', arch: 'arm64', expected: 'rari-darwin-arm64' },
        { platform: 'linux', arch: 'x64', expected: 'rari-linux-x64' },
        { platform: 'linux', arch: 'arm64', expected: 'rari-linux-arm64' },
        { platform: 'win32', arch: 'x64', expected: 'rari-win32-x64' },
      ]

      for (const { platform, arch, expected } of platforms) {
        vi.spyOn(process, 'platform', 'get').mockReturnValue(platform as any)
        vi.spyOn(process, 'arch', 'get').mockReturnValue(arch as any)

        const instructions = getInstallationInstructions()
        expect(instructions).toContain(expected)
      }
    })
  })

  describe('error messages', () => {
    it('should provide helpful error message for unsupported platform', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('sunos' as any)
      vi.spyOn(process, 'arch', 'get').mockReturnValue('x64')

      expect(() => getInstallationInstructions()).toThrow(
        /Unsupported platform: sunos.*rari supports Linux, macOS, and Windows/,
      )
    })

    it('should provide helpful error message for unsupported architecture', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('linux')
      vi.spyOn(process, 'arch', 'get').mockReturnValue('s390x' as any)

      expect(() => getInstallationInstructions()).toThrow(
        /Unsupported architecture: s390x.*rari supports x64 and ARM64/,
      )
    })

    it('should mention supported platforms in error', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('aix' as any)
      vi.spyOn(process, 'arch', 'get').mockReturnValue('x64')

      expect(() => getInstallationInstructions()).toThrow(/Linux, macOS, and Windows/)
    })

    it('should mention supported architectures in error', () => {
      vi.spyOn(process, 'platform', 'get').mockReturnValue('darwin')
      vi.spyOn(process, 'arch', 'get').mockReturnValue('ppc64' as any)

      expect(() => getInstallationInstructions()).toThrow(/x64 and ARM64/)
    })
  })

  describe('getBinaryPath', () => {
    it('should find binary in workspace', () => {
      const binaryPath = getBinaryPath()
      const expectedPlatform = `rari-${process.platform}-${process.arch}`
      const expectedBinaryName = process.platform === 'win32' ? 'rari.exe' : 'rari'

      expect(typeof binaryPath).toBe('string')
      expect(binaryPath).toContain(expectedPlatform)
      expect(binaryPath).toContain(`${nodePath.sep}bin${nodePath.sep}${expectedBinaryName}`)
    })

    it('should return valid path that exists', () => {
      const binaryPath = getBinaryPath()
      const expectedBinaryName = process.platform === 'win32' ? 'rari.exe' : 'rari'

      expect(nodePath.isAbsolute(binaryPath)).toBe(true)
      expect(binaryPath.endsWith(expectedBinaryName)).toBe(true)
    })
  })
})
