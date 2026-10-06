import { expect, test } from '@playwright/test'

// Conformance for Qwik Router features served through the rari host. Each is a
// small ported Qwik behaviour; together they are the adapter's scoreboard.

test.describe('SSR + routing', () => {
  test('home renders routeLoader$ + inline server$ data', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('h1')).toHaveText('hello from qwik on rari')
    await expect(page.getByText('loaded-on-the-server')).toBeVisible()
    await expect(page.getByText('server-fn-ran')).toBeVisible()
  })

  test('DocumentHead sets the title', async ({ page }) => {
    await page.goto('/')
    await expect(page).toHaveTitle(/Home — rari \+ qwik/)
  })

  test('static + dynamic routes resolve', async ({ page }) => {
    expect((await page.goto('/about'))?.status()).toBe(200)
    const blog = await page.goto('/blog/hello-world')
    expect(blog?.status()).toBe(200)
    await expect(page.locator('h1')).toHaveText('blog post: hello-world')
    await expect(page).toHaveTitle('Blog: hello-world')
  })

  test('unknown route is a host 404 without rendering', async ({ request }) => {
    // rari owns routing: no manifest match and no 404 page means the host
    // answers directly; the guest never runs.
    const response = await request.get('/no-such-route')
    expect(response.status()).toBe(404)
    expect(response.headers()['x-rari-route']).toBe('miss')
  })

  test('rari resolves the route before Qwik renders', async ({ request }) => {
    const home = await request.get('/')
    expect(home.headers()['x-rari-route']).toBe('/')
    const post = await request.get('/blog/hello-world/')
    expect(post.headers()['x-rari-route']).toBe('/blog/[slug]')
    expect(await post.text()).toContain('blog post: hello-world')
  })

  test('Qwik route-file modifiers and markdown pages are host routes', async ({ request }) => {
    const narrow = await request.get('/narrow/')
    expect(narrow.status()).toBe(200)
    expect(narrow.headers()['x-rari-route']).toBe('/narrow')
    expect(await narrow.text()).toContain('narrow page via named layout')
    expect(await narrow.text()).toContain('data-layout="narrow"')

    const notes = await request.get('/notes/')
    expect(notes.status()).toBe(200)
    expect(notes.headers()['x-rari-route']).toBe('/notes')
    expect(await notes.text()).toContain('notes page from markdown')
  })

  test('route params come from the host match', async ({ page }) => {
    await page.goto('/blog/host-routed/')
    await expect(page.getByTestId('host-route')).toHaveText('/blog/[slug] host-routed')
  })

  test('trailing slash redirects like Qwik Router', async ({ request }) => {
    const response = await request.get('/about', { maxRedirects: 0 })
    expect(response.status()).toBe(301)
    expect(response.headers().location).toBe('/about/')
  })

  test('streams the shell before out-of-order content', async ({ page }) => {
    await page.goto('/slow')
    await expect(page.getByText('shell rendered immediately')).toBeVisible()
    await expect(page.getByText('streamed-in-out-of-order')).toBeVisible({ timeout: 5000 })
  })
})

test.describe('routeAction$', () => {
  test('Form submits and shows the action result', async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', error => errors.push(error.message))

    await page.goto('/')
    await page.getByRole('textbox').first().fill('caveman')
    await page.getByRole('button', { name: 'echo' }).click()

    await expect(page.getByText('you said: caveman')).toBeVisible({ timeout: 5000 })
    expect(errors).toEqual([])
  })

  test('Form posts work without JavaScript', async ({ browser }) => {
    const context = await browser.newContext({ javaScriptEnabled: false })
    const page = await context.newPage()
    await page.goto('/')
    await page.getByRole('textbox').first().fill('no-js')
    await page.getByRole('button', { name: 'echo' }).click()
    await expect(page.getByText('you said: no-js')).toBeVisible()
    await context.close()
  })
})
