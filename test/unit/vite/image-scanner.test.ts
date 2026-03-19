import fs from 'node:fs/promises'
import { scanForImageUsage } from '@rari/vite/image-scanner'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

vi.mock('node:fs/promises')
vi.mock('rolldown')

describe('image-scanner', () => {
  const mockSrcDir = '/test/src'

  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('scanForImageUsage', () => {
    it('should throw error when source directory does not exist', async () => {
      vi.mocked(fs.access).mockRejectedValue(Object.assign(new Error('ENOENT'), { code: 'ENOENT' }))

      await expect(scanForImageUsage(mockSrcDir)).rejects.toThrow(
        'Required source directory does not exist',
      )
    })

    it('should return empty manifest when no images found', async () => {
      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([])

      const result = await scanForImageUsage(mockSrcDir)

      expect(result).toEqual({ images: [] })
    })

    it('should scan directory and find image usages', async () => {
      const mockFile = 'Component.tsx'
      const mockContent = `
import Image from 'rari/image'

export default function MyComponent() {
  return <Image src="/test.jpg" width={800} quality={90} preload />
}
`

      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([
        { name: mockFile, isFile: () => true, isDirectory: () => false } as any,
      ])
      vi.mocked(fs.readFile).mockResolvedValue(mockContent)

      const result = await scanForImageUsage(mockSrcDir)

      expect(result.images).toHaveLength(1)
      expect(result.images[0]).toMatchObject({
        src: '/test.jpg',
        width: 800,
        quality: 90,
        preload: true,
      })
    })

    it('should handle named imports with alias', async () => {
      const mockContent = `
import { Image as Img } from 'rari/image'

export default function MyComponent() {
  return <Img src="/photo.png" width={600} />
}
`

      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([
        { name: 'Test.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])
      vi.mocked(fs.readFile).mockResolvedValue(mockContent)

      const result = await scanForImageUsage(mockSrcDir)

      expect(result.images).toHaveLength(1)
      expect(result.images[0].src).toBe('/photo.png')
      expect(result.images[0].width).toBe(600)
    })

    it('should skip node_modules and dist directories', async () => {
      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce([
          { name: 'node_modules', isFile: () => false, isDirectory: () => true } as any,
          { name: 'dist', isFile: () => false, isDirectory: () => true } as any,
          { name: 'src', isFile: () => false, isDirectory: () => true } as any,
        ])
        .mockResolvedValueOnce([])

      await scanForImageUsage(mockSrcDir)

      expect(fs.readdir).toHaveBeenCalledTimes(2)
    })

    it('should handle multiple image components in same file', async () => {
      const mockContent = `
import Image from 'rari/image'

export default function Gallery() {
  return (
    <>
      <Image src="/img1.jpg" width={400} />
      <Image src="/img2.jpg" width={600} quality={80} />
      <Image src="/img3.jpg" preload />
    </>
  )
}
`

      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([
        { name: 'Gallery.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])
      vi.mocked(fs.readFile).mockResolvedValue(mockContent)

      const result = await scanForImageUsage(mockSrcDir)

      expect(result.images).toHaveLength(3)
    })

    it('should handle images with http URLs', async () => {
      const mockContent = `
import Image from 'rari/image'

export default function RemoteImage() {
  return <Image src="https://example.com/image.jpg" width={800} />
}
`

      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([
        { name: 'Remote.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])
      vi.mocked(fs.readFile).mockResolvedValue(mockContent)

      const result = await scanForImageUsage(mockSrcDir)

      expect(result.images).toHaveLength(1)
      expect(result.images[0].src).toBe('https://example.com/image.jpg')
    })

    it('should skip images with dynamic src', async () => {
      const mockContent = `
import Image from 'rari/image'

export default function DynamicImage({ src }) {
  return <Image src={src} width={800} />
}
`

      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([
        { name: 'Dynamic.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])
      vi.mocked(fs.readFile).mockResolvedValue(mockContent)

      const result = await scanForImageUsage(mockSrcDir)

      expect(result.images).toHaveLength(0)
    })

    it('should handle preload false correctly', async () => {
      const mockContent = `
import Image from 'rari/image'

export default function NoPreload() {
  return <Image src="/test.jpg" width={800} preload={false} />
}
`

      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([
        { name: 'NoPreload.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])
      vi.mocked(fs.readFile).mockResolvedValue(mockContent)

      const result = await scanForImageUsage(mockSrcDir)

      expect(result.images).toHaveLength(1)
      expect(result.images[0].preload).toBe(false)
    })

    it('should scan additional directories', async () => {
      const additionalDir = '/test/components'

      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([])

      await scanForImageUsage(mockSrcDir, [additionalDir])

      expect(fs.access).toHaveBeenCalledWith(mockSrcDir)
      expect(fs.access).toHaveBeenCalledWith(additionalDir)
    })

    it('should handle file read errors gracefully', async () => {
      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([
        { name: 'Error.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])
      vi.mocked(fs.readFile).mockRejectedValue(new Error('Permission denied'))

      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})

      const result = await scanForImageUsage(mockSrcDir)

      expect(result.images).toHaveLength(0)
      expect(consoleSpy).toHaveBeenCalled()
    })

    it('should deduplicate images with same src, width, and quality', async () => {
      const mockContent = `
import Image from 'rari/image'

export default function Duplicate() {
  return (
    <>
      <Image src="/same.jpg" width={800} quality={75} />
      <Image src="/same.jpg" width={800} quality={75} />
    </>
  )
}
`

      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([
        { name: 'Dup.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])
      vi.mocked(fs.readFile).mockResolvedValue(mockContent)

      const result = await scanForImageUsage(mockSrcDir)

      expect(result.images).toHaveLength(1)
    })

    it('should handle nested directories', async () => {
      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir)
        .mockResolvedValueOnce([
          { name: 'components', isFile: () => false, isDirectory: () => true } as any,
        ])
        .mockResolvedValueOnce([
          { name: 'Image.tsx', isFile: () => true, isDirectory: () => false } as any,
        ])

      vi.mocked(fs.readFile).mockResolvedValue(`
import Image from 'rari/image'
export default function() { return <Image src="/nested.jpg" width={400} /> }
`)

      const result = await scanForImageUsage(mockSrcDir)

      expect(result.images).toHaveLength(1)
      expect(result.images[0].src).toBe('/nested.jpg')
    })

    it('should only process tsx, ts, jsx, js files', async () => {
      vi.mocked(fs.access).mockResolvedValue(undefined)
      vi.mocked(fs.readdir).mockResolvedValue([
        { name: 'Component.tsx', isFile: () => true, isDirectory: () => false } as any,
        { name: 'styles.css', isFile: () => true, isDirectory: () => false } as any,
        { name: 'data.json', isFile: () => true, isDirectory: () => false } as any,
        { name: 'README.md', isFile: () => true, isDirectory: () => false } as any,
      ])

      vi.mocked(fs.readFile).mockResolvedValue(`
import Image from 'rari/image'
export default function() { return <Image src="/test.jpg" /> }
`)

      await scanForImageUsage(mockSrcDir)

      expect(fs.readFile).toHaveBeenCalledTimes(1)
      expect(fs.readFile).toHaveBeenCalledWith(
        expect.stringContaining('Component.tsx'),
        'utf8',
      )
    })
  })
})
