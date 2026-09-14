import { expect, test } from '@playwright/test'

interface SoftNavSample {
  readonly path: string
  readonly h1: string | null
  readonly animNames: readonly string[]
  readonly hasPageExit: boolean
  readonly hasPageEnter: boolean
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
        const collect = () => {
          const names = Array.from(new Set(
            document.getAnimations({ subtree: true })
              .map((animation) => String(animation.animationName || ''))
              .filter((name) => name !== '' && name !== 'none')
          )).sort();
          return names;
        };
        let frames = 0;
        const tick = () => {
          frames += 1;
          const names = collect();
          const hasPage = names.some((name) => name.indexOf('rari-page-') !== -1);
          if (hasPage || frames >= 12) {
            softNav.samples.push({
              path: location.pathname,
              h1: (document.querySelector('main h1, h1') && document.querySelector('main h1, h1').textContent)
                ? document.querySelector('main h1, h1').textContent.trim()
                : null,
              animNames: names,
              hasPageExit: names.some((name) => name.indexOf('rari-page-exit') !== -1),
              hasPageEnter: names.some((name) => name.indexOf('rari-page-enter') !== -1),
              hasUaGroup: names.some((name) => name.indexOf('-ua-view-transition-group') !== -1),
              urlHeldUntilCommit: softNav.pathBeforeCommit === softNav.pathAtStart,
              pathBeforeCommit: softNav.pathBeforeCommit,
              pathAtCommit: softNav.pathAtCommit,
            });
            return;
          }
          requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      });
    })()`)
  })

  for (const route of ROUTES) {
    test(`soft-nav to ${route.href} updates content with the same page VT`, async ({ page }) => {
      await page.goto('/')
      await expect(page.locator('h1')).toContainText('Test App Home')

      const nav = page.getByTestId('site-nav')
      await nav.evaluate(node => {
        node.setAttribute('data-persist', 'nav-ok')
      })

      await page.evaluate(() => {
        const softNav = (window as SoftNavWindow).__softNav
        if (softNav) softNav.samples = []
      })

      await page.locator(`nav a[href="${route.href}"]`).first().click()
      await expect(page).toHaveURL(route.href)
      await expect(page.locator('h1')).toHaveText(route.h1)

      await expect
        .poll(async () =>
          page.evaluate(() => (window as SoftNavWindow).__softNav?.samples.length ?? 0),
        )
        .toBeGreaterThan(0)

      const sample = await page.evaluate(() => (window as SoftNavWindow).__softNav!.samples.at(-1)!)

      expect(
        sample.hasPageExit,
        `exit missing for ${route.href}: ${sample.animNames.join(', ')}`,
      ).toBe(true)
      expect(
        sample.hasPageEnter,
        `enter missing for ${route.href}: ${sample.animNames.join(', ')}`,
      ).toBe(true)
      expect(sample.hasUaGroup, `UA group morph leaked for ${route.href}`).toBe(false)
      expect(sample.urlHeldUntilCommit).toBe(true)
      expect(sample.pathBeforeCommit).toBe('/')
      expect(sample.pathAtCommit).toBe(route.href)
      await expect(nav).toHaveAttribute('data-persist', 'nav-ok')
    })
  }

  test('page VT animation set is identical across routes', async ({ page }) => {
    const signatures: string[] = []

    for (const route of ROUTES) {
      await page.goto('/')
      await page.evaluate(() => {
        const softNav = (window as SoftNavWindow).__softNav
        if (softNav) softNav.samples = []
      })
      await page.locator(`nav a[href="${route.href}"]`).first().click()
      await expect(page).toHaveURL(route.href)
      await expect(page.locator('h1')).toHaveText(route.h1)
      await expect
        .poll(async () =>
          page.evaluate(() => (window as SoftNavWindow).__softNav?.samples.length ?? 0),
        )
        .toBeGreaterThan(0)

      const sample = await page.evaluate(() => (window as SoftNavWindow).__softNav!.samples.at(-1)!)
      signatures.push(
        sample.animNames
          .filter(name => name.includes('rari-page-'))
          .sort()
          .join('|'),
      )
    }

    expect(new Set(signatures).size).toBe(1)
    expect(signatures[0]).toContain('rari-page-enter')
    expect(signatures[0]).toContain('rari-page-exit')
  })
})
