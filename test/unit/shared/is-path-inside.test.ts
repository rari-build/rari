import { isPathInside, toPosixPath } from '@rari/shared/utils/path'
import { describe, expect, it } from 'vite-plus/test'

describe('isPathInside', () => {
  it('matches when Vite forwards slashes and path.resolve uses backslashes', () => {
    const appDir = String.raw`C:\Users\dev\project\src\app`
    const viteFile = 'C:/Users/dev/project/src/app/page.tsx'

    expect(viteFile.startsWith(appDir)).toBe(false)
    expect(isPathInside(viteFile, appDir)).toBe(true)
    expect(isPathInside(`${toPosixPath(appDir)}/blog/page.tsx`, appDir)).toBe(true)
    expect(isPathInside('C:/Users/dev/project/src/components/Welcome.tsx', appDir)).toBe(false)
  })

  it('matches posix paths on either side', () => {
    expect(isPathInside('/proj/src/app/page.tsx', '/proj/src/app')).toBe(true)
    expect(isPathInside('/proj/src/app', '/proj/src/app')).toBe(true)
    expect(isPathInside('/proj/src/apparent/page.tsx', '/proj/src/app')).toBe(false)
  })
})
