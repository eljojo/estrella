import { test, expect } from '@playwright/test'

test.describe('Weave', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    // Navigate to Weave tab
    await page.click('button:has-text("Weave")')
  })

  test('should display weave form', async ({ page }) => {
    // Check that the weave form is visible
    await expect(page.locator('label[for="weave-length"]')).toBeVisible()
    await expect(page.locator('label[for="crossfade"]')).toBeVisible()
    await expect(page.locator('label[for="curve"]')).toBeVisible()
    await expect(page.locator('label[for="weave-dither"]')).toBeVisible()
  })

  test('should have add pattern dropdown', async ({ page }) => {
    // The add pattern dropdown should be visible
    const addSelect = page.locator('select.weave-add-select')
    await expect(addSelect).toBeVisible()
    await expect(addSelect).toHaveValue('')
  })

  test('should add a pattern when selecting from dropdown', async ({ page }) => {
    // Select a pattern from the dropdown
    const addSelect = page.locator('select.weave-add-select')
    await addSelect.selectOption('ripple')

    // Should show the pattern in the list
    await expect(page.locator('.weave-entry')).toBeVisible()
    await expect(page.locator('.weave-entry-name')).toContainText('ripple')
  })

  test('should add multiple patterns', async ({ page }) => {
    // Add first pattern
    await page.locator('select.weave-add-select').selectOption('ripple')
    await expect(page.locator('.weave-entry')).toHaveCount(1)

    // Add second pattern
    await page.locator('select.weave-add-select').selectOption('waves')
    await expect(page.locator('.weave-entry')).toHaveCount(2)

    // Add third pattern
    await page.locator('select.weave-add-select').selectOption('plasma')
    await expect(page.locator('.weave-entry')).toHaveCount(3)
  })

  test('should remove a pattern when clicking remove button', async ({ page }) => {
    // Add a pattern
    await page.locator('select.weave-add-select').selectOption('ripple')
    await expect(page.locator('.weave-entry')).toHaveCount(1)

    // Click remove button
    await page.click('.weave-entry-actions button[title="Remove"]')

    // Pattern should be removed
    await expect(page.locator('.weave-entry')).toHaveCount(0)
  })

  test('should show pattern parameters when expanded', async ({ page }) => {
    // Add a pattern with parameters
    await page.locator('select.weave-add-select').selectOption('ripple')

    // Should show params grid (ripple has parameters)
    await expect(page.locator('.weave-entry-params')).toBeVisible()
  })

  test('should collapse and expand pattern', async ({ page }) => {
    // Add a pattern
    await page.locator('select.weave-add-select').selectOption('ripple')

    // Params should be visible initially
    await expect(page.locator('.weave-entry-params')).toBeVisible()

    // Click header to collapse
    await page.click('.weave-entry-header')

    // Params should be hidden
    await expect(page.locator('.weave-entry-params')).not.toBeVisible()

    // Click again to expand
    await page.click('.weave-entry-header')

    // Params should be visible again
    await expect(page.locator('.weave-entry-params')).toBeVisible()
  })

  test('should show preview after adding 2 patterns', async ({ page }) => {
    // Preview should show placeholder initially
    await expect(page.locator('.preview-placeholder-text')).toBeVisible()

    // Add first pattern
    await page.locator('select.weave-add-select').selectOption('ripple')

    // Still no preview with just 1 pattern
    await expect(page.locator('.preview-placeholder-text')).toBeVisible()

    // Add second pattern
    await page.locator('select.weave-add-select').selectOption('waves')

    // Preview should appear
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })
  })

  test('should disable print button with less than 2 patterns', async ({ page }) => {
    const printButton = page.locator('button:has-text("Print")')

    // Print button should be disabled with 0 patterns
    await expect(printButton).toBeDisabled()

    // Add one pattern
    await page.locator('select.weave-add-select').selectOption('ripple')

    // Still disabled with 1 pattern
    await expect(printButton).toBeDisabled()

    // Add second pattern
    await page.locator('select.weave-add-select').selectOption('waves')

    // Now should be enabled
    await expect(printButton).toBeEnabled()
  })

  test('should have randomize button for each pattern', async ({ page }) => {
    // Add a pattern
    await page.locator('select.weave-add-select').selectOption('ripple')

    // Randomize button should be visible
    await expect(page.locator('.weave-entry-actions button[title="Randomize"]')).toBeVisible()
  })

  test('should have randomize all button', async ({ page }) => {
    const randomizeAllButton = page.locator('button:has-text("Randomize All")')

    // Button should be disabled with no patterns
    await expect(randomizeAllButton).toBeDisabled()

    // Add a pattern
    await page.locator('select.weave-add-select').selectOption('ripple')

    // Button should be enabled
    await expect(randomizeAllButton).toBeEnabled()
  })

  test('no console errors when adding patterns', async ({ page }) => {
    const consoleErrors: string[] = []
    page.on('pageerror', (err) => consoleErrors.push(err.message))

    // Add multiple patterns
    await page.locator('select.weave-add-select').selectOption('ripple')
    await page.waitForTimeout(500)

    await page.locator('select.weave-add-select').selectOption('waves')
    await page.waitForTimeout(500)

    await page.locator('select.weave-add-select').selectOption('plasma')
    await page.waitForTimeout(500)

    // Check for errors
    expect(consoleErrors.length).toBe(0)
  })
})
