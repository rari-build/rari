import { getBinaryPath, getInstallationInstructions } from '@rari/platform'
import { afterEach, describe, expect, it } from 'vitest'

describe('platform', () => {
  const originalPlatform = process.platform
  const originalArch = process.arch

  afterEach(() => {
    Object.defineProperty(process, 'platform', {
      value: originalPlatform,
      writable: true,
      configurable: true,
    })
    Object.defineProperty(process, 'arch', {
      value: originalArch,
      writable: true,
      configurable: true,
    })
  })

  describe('getInstallationInstructions', () => {
    it('should return installation instructions for darwin-arm64', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      Object.defineProperty(process, 'arch', { value: 'arm64' })

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-darwin-arm64')
      expect(instructions).toContain('npm install')
      expect(instructions).toContain('pnpm add')
      expect(instructions).toContain('yarn add')
      expect(instructions).toContain('cargo install')
    })

    it('should return installation instructions for darwin-x64', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-darwin-x64')
    })

    it('should return installation instructions for linux-x64', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-linux-x64')
    })

    it('should return installation instructions for linux-arm64', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      Object.defineProperty(process, 'arch', { value: 'arm64' })

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-linux-arm64')
    })

    it('should return installation instructions for win32-x64', () => {
      Object.defineProperty(process, 'platform', { value: 'win32' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      const instructions = getInstallationInstructions()

      expect(instructions).toContain('rari-win32-x64')
    })

    it('should throw error for unsupported platform', () => {
      Object.defineProperty(process, 'platform', { value: 'freebsd' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      expect(() => getInstallationInstructions()).toThrow(
        'Unsupported platform: freebsd',
      )
    })

    it('should throw error for unsupported architecture', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      Object.defineProperty(process, 'arch', { value: 'ia32' })

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
        Object.defineProperty(process, 'platform', { value: platform })
        Object.defineProperty(process, 'arch', { value: arch })

        const instructions = getInstallationInstructions()
        expect(instructions).toContain(expected)
      }
    })
  })

  describe('error messages', () => {
    it('should provide helpful error message for unsupported platform', () => {
      Object.defineProperty(process, 'platform', { value: 'sunos' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      expect(() => getInstallationInstructions()).toThrow(
        /Unsupported platform: sunos.*rari supports Linux, macOS, and Windows/,
      )
    })

    it('should provide helpful error message for unsupported architecture', () => {
      Object.defineProperty(process, 'platform', { value: 'linux' })
      Object.defineProperty(process, 'arch', { value: 's390x' })

      expect(() => getInstallationInstructions()).toThrow(
        /Unsupported architecture: s390x.*rari supports x64 and ARM64/,
      )
    })

    it('should mention supported platforms in error', () => {
      Object.defineProperty(process, 'platform', { value: 'aix' })
      Object.defineProperty(process, 'arch', { value: 'x64' })

      expect(() => getInstallationInstructions()).toThrow(/Linux, macOS, and Windows/)
    })

    it('should mention supported architectures in error', () => {
      Object.defineProperty(process, 'platform', { value: 'darwin' })
      Object.defineProperty(process, 'arch', { value: 'ppc64' })

      expect(() => getInstallationInstructions()).toThrow(/x64 and ARM64/)
    })
  })

  describe('getBinaryPath', () => {
    it('should find binary in workspace', () => {
      const path = getBinaryPath()
      const expectedPlatform = `rari-${process.platform}-${process.arch}`
      const expectedBinaryName = process.platform === 'win32' ? 'rari.exe' : 'rari'

      expect(typeof path).toBe('string')
      expect(path).toContain(expectedPlatform)
      expect(path).toContain(`${require('node:path').sep}bin${require('node:path').sep}${expectedBinaryName}`)
    })

    it('should return valid path that exists', () => {
      const path = getBinaryPath()
      const expectedBinaryName = process.platform === 'win32' ? 'rari.exe' : 'rari'

      expect(require('node:path').isAbsolute(path)).toBe(true)
      expect(path.endsWith(expectedBinaryName)).toBe(true)
    })
  })
})
