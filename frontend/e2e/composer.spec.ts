import { test, expect } from '@playwright/test'

test.describe('Document Editor', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    // Navigate to Editor tab
    await page.click('button:has-text("Editor")')
  })

  test('should display editor form', async ({ page }) => {
    // Check that the editor form is visible
    await expect(page.locator('label:has-text("Components")')).toBeVisible()
    // Add Component dropdown should be visible
    await expect(page.locator('.weave-add-select')).toBeVisible()
  })

  test('should show Components (0) initially', async ({ page }) => {
    await expect(page.locator('label:has-text("Components (0)")')).toBeVisible()
  })

  test('should add a text component from the dropdown', async ({ page }) => {
    await expect(page.locator('label:has-text("Components (0)")')).toBeVisible()

    // Select "Text" from the add component dropdown
    await page.selectOption('.weave-add-select', 'text')

    // Should now show 1 component
    await expect(page.locator('label:has-text("Components (1)")')).toBeVisible()

    // The component item should be visible in the list
    await expect(page.locator('.layer-item')).toBeVisible()
  })

  test('should show component editor after adding a component', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')

    // The editor section should appear
    await expect(page.locator('.selected-layer-editor')).toBeVisible()

    // Should have content textarea and style toggles
    await expect(page.locator('.component-editor')).toBeVisible()
  })

  test('should add multiple components of different types', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')
    await expect(page.locator('label:has-text("Components (1)")')).toBeVisible()

    await page.selectOption('.weave-add-select', 'divider')
    await expect(page.locator('label:has-text("Components (2)")')).toBeVisible()

    await page.selectOption('.weave-add-select', 'banner')
    await expect(page.locator('label:has-text("Components (3)")')).toBeVisible()

    // Should have 3 component items
    await expect(page.locator('.layer-item')).toHaveCount(3)
  })

  test('should remove a component when clicking delete button', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')
    await expect(page.locator('label:has-text("Components (1)")')).toBeVisible()

    // Click the delete button
    await page.click('.layer-item .icon-btn.delete')

    // Should now show 0 components
    await expect(page.locator('label:has-text("Components (0)")')).toBeVisible()
  })

  test('should select a component when clicking on it', async ({ page }) => {
    // Add two components
    await page.selectOption('.weave-add-select', 'text')
    await page.selectOption('.weave-add-select', 'divider')

    const items = page.locator('.layer-item')
    await expect(items).toHaveCount(2)

    // Click on the first component
    await items.first().click()

    // First component should be selected
    await expect(items.first()).toHaveClass(/selected/)
  })

  test('should update JSON when adding components', async ({ page }) => {
    // Open the advanced panel
    await page.click('summary:has-text("Advanced")')

    const textarea = page.locator('.json-editor textarea')

    // Initially should have empty document array
    const initialJson = await textarea.inputValue()
    expect(JSON.parse(initialJson).document).toHaveLength(0)

    // Add a component
    await page.selectOption('.weave-add-select', 'text')

    // JSON should now have 1 component
    const updatedJson = await textarea.inputValue()
    expect(JSON.parse(updatedJson).document).toHaveLength(1)
  })

  test('should enable print button after adding a component', async ({ page }) => {
    // Print button should be disabled initially
    const printButton = page.locator('button:has-text("Print")')
    await expect(printButton).toBeDisabled()

    // Add a component
    await page.selectOption('.weave-add-select', 'text')

    // Print button should now be enabled
    await expect(printButton).toBeEnabled()
  })

  test('should show preview after adding a component', async ({ page }) => {
    // Preview should show placeholder text initially
    await expect(page.locator('.preview-placeholder-text')).toBeVisible()

    // Add a text component
    await page.selectOption('.weave-add-select', 'text')

    // Wait for preview to load
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 10000 })
  })

  test('should show type-specific editor for text', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'text')

    // Text editor should show content, style toggles, size, font
    await expect(page.locator('.component-editor textarea')).toBeVisible()
    await expect(page.locator('.style-toggles')).toBeVisible()
  })

  test('should show type-specific editor for divider', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'divider')

    // Divider editor should show style dropdown
    await expect(page.locator('.component-editor select')).toBeVisible()
  })

  test('should add a canvas component with nested elements', async ({ page }) => {
    await page.selectOption('.weave-add-select', 'canvas')

    await expect(page.locator('label:has-text("Components (1)")')).toBeVisible()

    // Canvas editor should show height input and dither dropdown
    await expect(page.locator('.component-editor label:has-text("Height")')).toBeVisible()
    await expect(page.locator('.component-editor label:has-text("Dither")')).toBeVisible()

    // Should show Elements (0) label
    await expect(page.locator('.component-editor label:has-text("Elements (0)")')).toBeVisible()
  })
})
