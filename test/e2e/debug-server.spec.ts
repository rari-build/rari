import { expect, test } from '@playwright/test'

test.describe('Server Debugging', () => {
  test('should verify server is responding', async ({ page, request }) => {
    // Test if server root is accessible
    const rootResponse = await request.get('/')
    console.log(`Root response status: ${rootResponse.status()}`)
    console.log(`Root response headers:`, await rootResponse.headersArray())

    expect(rootResponse.ok() || rootResponse.status() === 304).toBeTruthy()

    // Test if error-test route exists
    const errorTestResponse = await request.get('/error-test')
    console.log(`/error-test response status: ${errorTestResponse.status()}`)

    if (!errorTestResponse.ok()) {
      const body = await errorTestResponse.text()
      console.log(`/error-test response body:`, body.substring(0, 500))
    }

    // Try to load the page and capture console logs
    const messages: string[] = []
    page.on('console', msg => {
      messages.push(`${msg.type()}: ${msg.text()}`)
    })

    page.on('pageerror', error => {
      console.log(`Page error: ${error.message}`)
    })

    try {
      await page.goto('/error-test', { waitUntil: 'networkidle', timeout: 15000 })
      const html = await page.content()
      console.log(`Page HTML length: ${html.length}`)
      console.log(`Page title: ${await page.title()}`)
      console.log(`Console messages:`, messages)

      // Check if the page has any content
      const bodyText = await page.locator('body').textContent()
      console.log(`Body text: ${bodyText?.substring(0, 200)}`)
    }
    catch (error) {
      console.log(`Navigation error: ${error}`)
      const html = await page.content()
      console.log(`Page HTML on error:`, html.substring(0, 1000))
      throw error
    }
  })
})
