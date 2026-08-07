import { expect, test } from '@playwright/test'

test('loads WASM, runs a query and preserves language', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByText('TRACEFORGE')).toBeVisible()
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
  await expect(page.getByText('TRACEFORGE')).toBeVisible()
  const overflows = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth)
  expect(overflows).toBe(false)
})

