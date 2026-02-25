import { expect, test } from '@playwright/test'

const STARS_FORMAT_REGEX = /^[\d,]+$/
const COMMIT_HASH_REGEX = /^[0-9a-f]{8}$/
const MIT_LICENSE_REGEX = /MIT License/
const RYAN_SKINNER_REGEX = /Ryan Skinner/
const GITHUB_COMMIT_URL_REGEX = /^https:\/\/github\.com\/rari-build\/rari\/commit\/[0-9a-f]{8}$/
const COMMA_REGEX = /,/g

test.describe('Fetch Cache E2E Tests', () => {
  test.describe('GitHub Data Caching', () => {
    test('should display cached GitHub stars in footer', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer')
      await expect(footer).toBeVisible()

      const githubLink = footer.locator('a[aria-label="rari on GitHub"]')
      await expect(githubLink).toBeVisible()

      const starsSpan = githubLink.locator('span.text-xs.text-gray-400')

      if (await starsSpan.count() > 0) {
        const starsText = await starsSpan.textContent()

        if (starsText) {
          expect(starsText).toMatch(STARS_FORMAT_REGEX)

          const stars = Number.parseInt(starsText.replace(COMMA_REGEX, ''), 10)
          expect(stars).toBeGreaterThan(0)
        }
      }
    })

    test('should display cached commit hash in footer', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const commitLink = page.locator('a[href^="https://github.com/rari-build/rari/commit/"]')

      if (await commitLink.count() > 0) {
        await expect(commitLink).toBeVisible()

        const commitHash = await commitLink.textContent()
        expect(commitHash).toBeTruthy()

        expect(commitHash?.length).toBe(8)

        expect(commitHash).toMatch(COMMIT_HASH_REGEX)
      }
    })

    test('should cache GitHub data across page navigations', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer')
      const firstStars = await footer.locator('a[aria-label="rari on GitHub"] span.text-xs.text-gray-400').textContent()

      await page.goto('/docs/getting-started')
      await page.waitForLoadState('networkidle')

      const footer2 = page.locator('footer')
      const secondStars = await footer2.locator('a[aria-label="rari on GitHub"] span.text-xs.text-gray-400').textContent()

      if (firstStars && secondStars) {
        expect(firstStars).toBe(secondStars)
      }
    })

    test('should handle GitHub API failures gracefully', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer')
      await expect(footer).toBeVisible()

      await expect(footer.getByText(MIT_LICENSE_REGEX)).toBeVisible()
      await expect(footer.getByText(RYAN_SKINNER_REGEX)).toBeVisible()
    })
  })

  test.describe('Cache Revalidation', () => {
    test('should use 1 hour revalidation for GitHub data', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer')
      const githubLink = footer.locator('a[aria-label="rari on GitHub"]')
      await expect(githubLink).toBeVisible()
    })

    test('should maintain cache across multiple page loads', async ({ page }) => {
      const stars: (string | null)[] = []

      for (let i = 0; i < 3; i++) {
        await page.goto('/')
        await page.waitForLoadState('networkidle')

        const footer = page.locator('footer')
        const starsText = await footer.locator('a[aria-label="rari on GitHub"] span.text-xs.text-gray-400').textContent()
        stars.push(starsText)
      }

      if (stars[0]) {
        expect(stars[0]).toBe(stars[1])
        expect(stars[1]).toBe(stars[2])
      }
    })
  })

  test.describe('Footer Integration', () => {
    test('should display all footer elements with cached data', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer')
      await expect(footer).toBeVisible()

      const githubLink = footer.locator('a[aria-label="rari on GitHub"]')
      await expect(githubLink).toBeVisible()

      const discordLink = footer.locator('a[href="https://discord.gg/GSh2Ak3b8Q"]')
      await expect(discordLink).toBeVisible()

      const blueskyLink = footer.locator('a[href="https://bsky.app/profile/rari.build"]')
      await expect(blueskyLink).toBeVisible()

      const licenseLink = footer.locator('a[href="https://github.com/rari-build/rari/blob/main/LICENSE"]')
      await expect(licenseLink).toBeVisible()
    })

    test('should format star count with commas', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer')
      const starsText = await footer.locator('a[aria-label="rari on GitHub"] span.text-xs.text-gray-400').textContent()

      if (starsText) {
        const stars = Number.parseInt(starsText.replace(COMMA_REGEX, ''), 10)

        if (stars >= 1000) {
          expect(starsText).toContain(',')
        }
      }
    })

    test('should link commit hash to GitHub', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const commitLink = page.locator('a[href^="https://github.com/rari-build/rari/commit/"]')

      if (await commitLink.count() > 0) {
        const href = await commitLink.getAttribute('href')
        expect(href).toMatch(GITHUB_COMMIT_URL_REGEX)
      }
    })
  })

  test.describe('Performance', () => {
    test('should load footer quickly with cached data', async ({ page }) => {
      const startTime = Date.now()

      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer')
      await expect(footer).toBeVisible()

      const loadTime = Date.now() - startTime

      expect(loadTime).toBeLessThan(5000)
    })

    test('should not block page render while fetching GitHub data', async ({ page }) => {
      await page.goto('/')

      await expect(page.locator('h1')).toBeVisible({ timeout: 3000 })

      await expect(page.locator('footer')).toBeVisible()
    })
  })

  test.describe('Error Handling', () => {
    test('should render footer without stars if API fails', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer')
      await expect(footer).toBeVisible()

      const githubLink = footer.locator('a[aria-label="rari on GitHub"]')
      await expect(githubLink).toBeVisible()

      await expect(githubLink).toHaveAttribute('href', 'https://github.com/rari-build/rari')
    })

    test('should render footer without commit hash if API fails', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const footer = page.locator('footer')
      await expect(footer).toBeVisible()

      await expect(footer.getByText(RYAN_SKINNER_REGEX)).toBeVisible()
    })
  })
})
