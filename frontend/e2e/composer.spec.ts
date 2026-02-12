import { test, expect } from '@playwright/test'

test.describe('Document Editor', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    // Wait for app to hydrate, then navigate to Editor tab
    await page.locator('button.tab:has-text("Editor")').waitFor()
    await page.click('button:has-text("Editor")')
    await page.locator('label:has-text("Components")').waitFor()

    // Clear all default components to start fresh
    while ((await page.locator('.layers-list > .layer-item').count()) > 0) {
      await page.locator('.layers-list > .layer-item').first().click()
      await page.locator('.layers-list > .layer-item .icon-btn.delete').first().click()
    }
    await expect(page.locator('label:has-text("Components (0)")')).toBeVisible()
  })

  test('should display editor form', async ({ page }) => {
    await expect(page.locator('label:has-text("Components")')).toBeVisible()
    await expect(page.locator('.weave-add-select')).toBeVisible()
  })

  test('should show Components (0) after clearing', async ({ page }) => {
    await expect(page.locator('label:has-text("Components (0)")')).toBeVisible()
  })

  test('should add a text component from the dropdown', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')

    await expect(page.locator('label:has-text("Components (1)")')).toBeVisible()
    await expect(page.locator('.layer-item')).toBeVisible()
  })

  test('should show component editor after adding a component', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')

    await expect(page.locator('.selected-layer-editor')).toBeVisible()
    await expect(page.locator('.component-editor')).toBeVisible()
  })

  test('should add multiple components of different types', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')
    await expect(page.locator('label:has-text("Components (1)")')).toBeVisible()

    await page.selectOption('.weave-add-select', 'divider')
    await expect(page.locator('label:has-text("Components (2)")')).toBeVisible()

    await page.selectOption('.weave-add-select', 'banner')
    await expect(page.locator('label:has-text("Components (3)")')).toBeVisible()

    await expect(page.locator('.layers-list > .layer-item')).toHaveCount(3)
  })

  test('should remove a component when clicking delete button', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')
    await expect(page.locator('label:has-text("Components (1)")')).toBeVisible()

    await page.locator('.layers-list > .layer-item .icon-btn.delete').first().click()

    await expect(page.locator('label:has-text("Components (0)")')).toBeVisible()
  })

  test('should select a component when clicking on it', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')
    await page.selectOption('.weave-add-select', 'divider')

    const items = page.locator('.layers-list > .layer-item')
    await expect(items).toHaveCount(2)

    await items.first().click()

    await expect(items.first()).toHaveClass(/selected/)
  })

  test('should update JSON when adding components', async ({ page }) => {
    await page.click('summary:has-text("Advanced")')

    const textarea = page.locator('.json-editor textarea')

    const initialJson = await textarea.inputValue()
    expect(JSON.parse(initialJson).document).toHaveLength(0)

    await page.selectOption('.weave-add-select', 'text')

    await expect(textarea).toHaveValue(/"type":\s*"text"/)
    const updatedJson = await textarea.inputValue()
    expect(JSON.parse(updatedJson).document).toHaveLength(1)
  })

  test('should enable print button after adding a component', async ({ page }) => {
    const printButton = page.locator('button:has-text("Print")')
    await expect(printButton).toBeDisabled()

    await page.selectOption('.weave-add-select', 'text')

    await expect(printButton).toBeEnabled()
  })

  test('should show preview after adding a component', async ({ page }) => {
    await expect(page.locator('.preview-placeholder-text')).toBeVisible()

    await page.selectOption('.weave-add-select', 'text')

    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 10000 })
  })

  test('should show type-specific editor for text', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')

    await expect(page.locator('.component-editor textarea')).toBeVisible()
    await expect(page.locator('.style-toggles')).toBeVisible()
  })

  test('should show type-specific editor for divider', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'divider')

    await expect(page.locator('.component-editor select')).toBeVisible()
  })

  test('should add a canvas component with nested elements', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'canvas')

    await expect(page.locator('label:has-text("Components (1)")')).toBeVisible()

    await expect(page.locator('.component-editor label:has-text("Height")')).toBeVisible()
    await expect(page.locator('.component-editor label:has-text("Dither")')).toBeVisible()

    await expect(page.locator('.component-editor label:has-text("Elements (0)")')).toBeVisible()
  })
})
