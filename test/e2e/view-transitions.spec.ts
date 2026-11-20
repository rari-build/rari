import type { Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { waitForRariRuntime } from './shared/helpers'

interface ViewTransitionSample {
  readonly path: string
  readonly types: readonly string[]
  readonly animations: readonly string[] | null
}

interface SoftNavSample {
  readonly path: string
  readonly h1: string | null
  readonly hasUaGroup: boolean
  readonly urlHeldUntilCommit: boolean
  readonly pathBeforeCommit: string | null
  readonly pathAtCommit: string | null
}

type SoftNavWindow = Window & {
  __viewTransitions?: ViewTransitionSample[]
  __softNav?: {
    samples: SoftNavSample[]
    pathAtStart: string | null
    pathBeforeCommit: string | null
    pathAtCommit: string | null
  }
}

async function resetSamples(page: Page): Promise<void> {
  await page.evaluate(() => {
    const w = window as SoftNavWindow
    if (w.__softNav) w.__softNav.samples = []
    if (w.__viewTransitions) w.__viewTransitions.length = 0
  })
}

async function expectPageViewTransition(page: Page, path: string): Promise<void> {
  await expect
    .poll(async () =>
      page.evaluate(
        target =>
          (window as SoftNavWindow).__viewTransitions?.find(
            sample => sample.path === target && sample.animations != null,
          ) ?? null,
        path,
      ),
    )
    .toMatchObject({
      types: expect.arrayContaining(['nav', 'nav-forward']),
      animations: expect.arrayContaining(['rari-page-exit', 'rari-page-enter']),
    })
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

      const viewTransitions = [];
      window.__viewTransitions = viewTransitions;
      const startViewTransition = Document.prototype.startViewTransition;
      if (typeof startViewTransition === 'function') {
        Document.prototype.startViewTransition = function (arg) {
          const transition = startViewTransition.call(this, arg);
          const sample = {
            path: location.pathname,
            types: arg && typeof arg === 'object' && arg.types ? Array.from(arg.types) : [],
            animations: null,
          };
          viewTransitions.push(sample);
          transition.ready
            .then(() => {
              sample.animations = Array.from(new Set(
                document.getAnimations()
                  .map((animation) => String(animation.animationName || ''))
                  .filter((name) => name !== '' && name !== 'none')
              ));
            })
            .catch(() => {
              sample.animations = [];
            });
          return transition;
        };
      }

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

      await resetSamples(page)

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
      await expectPageViewTransition(page, route.href)
    })
  }

  test('soft-nav from a non-root page runs both exit and enter animations', async ({ page }) => {
    await page.goto('/about')
    await waitForRariRuntime(page)
    await expect(page.locator('h1')).toHaveText(/About Page/)

    await resetSamples(page)

    await page.locator('nav a[href="/blog"]').first().click()
    await expect(page).toHaveURL('/blog')
    await expect(page.locator('h1')).toHaveText(/Blog/)

    await expectPageViewTransition(page, '/blog')
  })
})
