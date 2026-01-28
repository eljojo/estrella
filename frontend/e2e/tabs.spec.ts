import { test, expect } from '@playwright/test'

test.describe('Tab Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('should load with Receipt tab active by default', async ({ page }) => {
    const receiptTab = page.locator('button.tab:has-text("Receipt")')
    await expect(receiptTab).toHaveClass(/active/)
  })

  test('should switch to Patterns tab', async ({ page }) => {
    await page.click('button:has-text("Patterns")')
    const patternsTab = page.locator('button.tab:has-text("Patterns")')
    await expect(patternsTab).toHaveClass(/active/)
    // Patterns form should be visible
    await expect(page.locator('label[for="pattern"]')).toBeVisible()
  })

  test('should switch to Weave tab', async ({ page }) => {
    await page.click('button:has-text("Weave")')
    const weaveTab = page.locator('button.tab:has-text("Weave")')
    await expect(weaveTab).toHaveClass(/active/)
  })

  test('should switch to Editor tab', async ({ page }) => {
    await page.click('button:has-text("Editor")')
    const editorTab = page.locator('button.tab:has-text("Editor")')
    await expect(editorTab).toHaveClass(/active/)
    // Editor form should be visible
    await expect(page.locator('label:has-text("Components")')).toBeVisible()
  })

  test('should switch to Photos tab', async ({ page }) => {
    await page.click('button:has-text("Photos")')
    const photosTab = page.locator('button.tab:has-text("Photos")')
    await expect(photosTab).toHaveClass(/active/)
  })

  test('all tabs should be clickable', async ({ page }) => {
    const tabs = ['Receipt', 'Patterns', 'Weave', 'Editor', 'Photos']

    for (const tabName of tabs) {
      await page.click(`button.tab:has-text("${tabName}")`)
      const tab = page.locator(`button.tab:has-text("${tabName}")`)
      await expect(tab).toHaveClass(/active/)
    }
  })
})
