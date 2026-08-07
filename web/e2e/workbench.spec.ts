import { expect, test } from '@playwright/test'

test.beforeEach(async ({ page }) => {
  page.on('pageerror', (error) => console.error(`PAGE ERROR: ${error.message}`))
  page.on('console', (message) => {
    if (message.type() === 'error') console.error(`BROWSER ERROR: ${message.text()}`)
  })
})

test('loads WASM, runs a query and preserves language', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByRole('link', { name: 'TraceForge home' })).toBeVisible({ timeout: 15_000 })
  const editor = page.getByRole('textbox', { name: /Query|Consulta/ })
  await expect(editor).toBeVisible()
  await editor.fill('outcome:failure AND user:ana')
  await page.getByRole('button', { name: /Run query|Ejecutar consulta/ }).click()
  await expect(page.getByText(/indexed-posting-lists/)).toBeVisible()
  await page.getByRole('button', { name: /Cambiar a español|Switch to English/ }).click()
  await page.reload()
  await expect(page.getByRole('button', { name: /Ejecutar consulta|Run query/ })).toBeVisible()
})

test('has no horizontal page overflow', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByRole('link', { name: 'TraceForge home' })).toBeVisible({ timeout: 15_000 })
  const overflow = await page.evaluate(() => ({
    page: { scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth },
    elements: [...document.querySelectorAll<HTMLElement>('body *')]
      .filter((element) => {
        const rect = element.getBoundingClientRect()
        return rect.right > document.documentElement.clientWidth + 1 || rect.left < -1
      })
      .slice(0, 10)
      .map((element) => ({ tag: element.tagName, className: element.className, right: Math.round(element.getBoundingClientRect().right) })),
  }))
  expect(overflow.page.scrollWidth, JSON.stringify(overflow)).toBeLessThanOrEqual(overflow.page.clientWidth)
})
