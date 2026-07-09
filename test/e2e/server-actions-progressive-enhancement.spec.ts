import { expect, test } from '@playwright/test'
import { resetActionsFixture } from './shared/server-action-helpers'

function decodeHtmlEntities(value: string) {
  return value
    .replaceAll('&quot;', '"')
    .replaceAll('&#39;', '\'')
    .replaceAll('&lt;', '<')
    .replaceAll('&gt;', '>')
    .replaceAll('&amp;', '&')
}

function getTodoFormFieldsFromHtml(html: string) {
  const formMatch = html.match(/<form[^>]*action="([^"]*)"[^>]*>[\s\S]*?<\/form>/i)
  if (!formMatch)
    return { action: null as string | null, fields: [] as Array<{ name: string, value: string }> }

  const formHtml = formMatch[0]
  const fields: Array<{ name: string, value: string }> = []
  const inputPattern = /<input\b[^>]*>/gi

  for (const inputTag of formHtml.match(inputPattern) ?? []) {
    const name = inputTag.match(/\bname="([^"]+)"/i)?.[1]
    if (!name)
      continue

    const rawValue = inputTag.match(/\bvalue="([^"]*)"/i)?.[1] ?? ''
    fields.push({ name, value: decodeHtmlEntities(rawValue) })
  }

  return { action: formMatch[1], fields }
}

test.describe('Server Actions Progressive Enhancement (no JS)', () => {
  test('SSR emits real form action URL and Flight metadata', async ({ browser, baseURL }) => {
    const context = await browser.newContext({ javaScriptEnabled: false })
    const page = await context.newPage()

    await page.goto(`${baseURL}/actions`, { waitUntil: 'networkidle' })
    await expect(page.getByTestId('page-title')).toBeVisible({ timeout: 15_000 })

    const html = await page.content()
    const { action, fields } = getTodoFormFieldsFromHtml(html)

    expect(action).toBe('/actions')
    expect(html).not.toContain('javascript:throw')
    expect(html).toMatch(/\$ACTION_/)
    expect(fields.some(field => field.name === '$ACTION_REF_1')).toBe(true)

    await context.close()
  })

  test('works with JavaScript disabled', async ({ browser, baseURL }) => {
    const context = await browser.newContext({ javaScriptEnabled: false })
    const page = await context.newPage()

    await page.goto(`${baseURL}/actions`, { waitUntil: 'networkidle' })
    await expect(page.getByTestId('todo-count')).toHaveText('Total: 2', { timeout: 15_000 })

    await page.fill('[data-testid="todo-input"]', 'No-JS todo')
    await Promise.all([
      page.waitForURL('**/actions'),
      page.getByTestId('submit-button').click(),
    ])

    await expect(page.getByTestId('todo-count')).toHaveText('Total: 3', { timeout: 15_000 })
    await expect(page.getByTestId('todo-list')).toContainText('No-JS todo')

    await context.close()
  })
})

test.describe.configure({ mode: 'serial' })

test.describe('Server Actions Progressive Enhancement', () => {
  test.beforeEach(async ({ page }) => {
    test.setTimeout(60_000)
    await resetActionsFixture(page, { waitUntil: 'networkidle', assertCompletedTodo: false })
  })

  test('native form POST without rsc-action-id decodes via decodeAction', async ({ page, baseURL }) => {
    await expect(page.getByTestId('todo-count')).toHaveText('Total: 2', { timeout: 15_000 })

    const html = await page.request.get(`${baseURL}/actions`).then(response => response.text())
    const { action, fields } = getTodoFormFieldsFromHtml(html)
    expect(action).toBe('/actions')
    expect(fields.some(field => field.name === '$ACTION_REF_1')).toBe(true)
    if (!action)
      throw new Error('Missing form action URL in SSR HTML')

    await page.evaluate(({ fields, action, todoText }) => {
      const form = document.querySelector('[data-testid="todo-form"] form') as HTMLFormElement | null
      if (!form)
        throw new Error('Todo form not found')

      const textInput = form.querySelector('input[name="text"]') as HTMLInputElement | null
      if (!textInput)
        throw new Error('Todo text input not found')

      form.action = action
      form.method = 'POST'
      form.enctype = 'multipart/form-data'

      for (const field of fields) {
        let input = form.querySelector(`input[name="${field.name}"]`) as HTMLInputElement | null
        if (!input) {
          input = document.createElement('input')
          input.type = 'hidden'
          input.name = field.name
          form.appendChild(input)
        }
        input.value = field.value
      }

      textInput.value = todoText
      // submit() bypasses React's onSubmit handler so the browser performs a native POST
      // (decodeAction path) instead of the client RPC path (rsc-action-id header).
      form.submit()
    }, { fields, action, todoText: 'Integration test todo' })

    await page.waitForURL('**/actions', { waitUntil: 'networkidle' })
    await expect(page.getByTestId('todo-count')).toHaveText('Total: 3', { timeout: 15_000 })
    await expect(page.getByTestId('todo-list')).toContainText('Integration test todo')
  })
})
