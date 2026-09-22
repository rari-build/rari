import { expect, test } from '@playwright/test'
import { waitForRariRuntime } from './shared/helpers'

interface SoftNavSample {
  readonly path: string
  readonly h1: string | null
  readonly hasUaGroup: boolean
  readonly urlHeldUntilCommit: boolean
  readonly pathBeforeCommit: string | null
  readonly pathAtCommit: string | null
}

type SoftNavWindow = Window & {
  __softNav?: {
    samples: SoftNavSample[]
    pathAtStart: string | null
    pathBeforeCommit: string | null
    pathAtCommit: string | null
  }
}

const ROUTES = [
  { href: '/about', h1: /About Page/ },
  { href: '/nested', h1: /Nested/ },
  { href: '/blog', h1: /Blog/ },
] as const

test.describe('Soft-nav view transitions', () => {
  test.describe.configure({ mode: 'serial' })

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(`(() => {
      const softNav = {
        samples: [],
        pathAtStart: null,
        pathBeforeCommit: null,
        pathAtCommit: null,
      };
      window.__softNav = softNav;

      window.addEventListener('rari:navigation-start', () => {
        softNav.pathAtStart = location.pathname;
        softNav.pathBeforeCommit = null;
        softNav.pathAtCommit = null;
      });

      window.addEventListener('rari:navigate', () => {
        softNav.pathBeforeCommit = location.pathname;
      });

      window.addEventListener('rari:navigate-committed', () => {
        softNav.pathAtCommit = location.pathname;
        let frames = 0;
        const tick = () => {
          frames += 1;
          if (frames < 2) {
            requestAnimationFrame(tick);
            return;
          }
          const names = Array.from(new Set(
            document.getAnimations({ subtree: true })
              .map((animation) => String(animation.animationName || ''))
              .filter((name) => name !== '' && name !== 'none')
          ));
          softNav.samples.push({
            path: location.pathname,
            h1: (document.querySelector('main h1, h1') && document.querySelector('main h1, h1').textContent)
              ? document.querySelector('main h1, h1').textContent.trim()
              : null,
            hasUaGroup: names.some((name) => name.indexOf('-ua-view-transition-group') !== -1),
            urlHeldUntilCommit: softNav.pathBeforeCommit === softNav.pathAtStart,
            pathBeforeCommit: softNav.pathBeforeCommit,
            pathAtCommit: softNav.pathAtCommit,
          });
        };
        requestAnimationFrame(tick);
      });
    })()`)
  })

  for (const route of ROUTES) {
    test(`soft-nav to ${route.href} commits after fetch without UA morph`, async ({ page }) => {
      await page.goto('/')
      await waitForRariRuntime(page)
      await expect(page.locator('h1')).toContainText('Test App Home')
      await expect(page.getByTestId('site-nav')).toBeVisible()

      await page.evaluate(() => {
        const softNav = (window as SoftNavWindow).__softNav
        if (softNav) softNav.samples = []
      })

      await page.locator(`nav a[href="${route.href}"]`).first().click()
      await expect(page).toHaveURL(route.href)
      await expect(page.locator('h1')).toHaveText(route.h1)
      await expect(page.getByTestId('site-nav')).toBeVisible()

      await expect
        .poll(async () =>
          page.evaluate(() => (window as SoftNavWindow).__softNav?.samples.length ?? 0),
        )
        .toBeGreaterThan(0)

      const sample = await page.evaluate(() => (window as SoftNavWindow).__softNav!.samples.at(-1)!)

      expect(sample.hasUaGroup, `UA group morph leaked for ${route.href}`).toBe(false)
      expect(sample.urlHeldUntilCommit).toBe(true)
      expect(sample.pathBeforeCommit).toBe('/')
      expect(sample.pathAtCommit).toBe(route.href)
    })
  }
})
