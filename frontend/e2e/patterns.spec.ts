import { test, expect } from '@playwright/test'

test('Patterns tab should work', async ({ page }) => {
  const consoleErrors: string[] = []
  page.on('pageerror', err => consoleErrors.push(err.message))

  await page.goto('/')
  await page.click('button:has-text("Patterns")')
  await page.waitForTimeout(500)

  // Should show pattern selector (use specific selector to avoid matching "Pattern Parameters")
  await expect(page.locator('label[for="pattern"]')).toBeVisible()

  // Check for errors
  console.log('Console errors:', consoleErrors)
  expect(consoleErrors.length).toBe(0)
})
