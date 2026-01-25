import { test, expect } from '@playwright/test'

test.describe('Composer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    // Navigate to Composer tab
    await page.click('button:has-text("Composer")')
  })

  test('should display composer form', async ({ page }) => {
    // Check that the composer form is visible
    await expect(page.locator('label:has-text("Canvas Height")')).toBeVisible()
    await expect(page.locator('label:has-text("Background")')).toBeVisible()
    await expect(page.locator('label:has-text("Dithering")')).toBeVisible()
    await expect(page.locator('label:has-text("Layers")')).toBeVisible()
  })

  test('should show Layers (0) initially', async ({ page }) => {
    await expect(page.locator('label:has-text("Layers (0)")')).toBeVisible()
  })

  test('should add a layer when clicking Add Layer button', async ({ page }) => {
    // Initially should show 0 layers
    await expect(page.locator('label:has-text("Layers (0)")')).toBeVisible()

    // Click the Add Layer button
    await page.click('button:has-text("+ Add Layer")')

    // Should now show 1 layer
    await expect(page.locator('label:has-text("Layers (1)")')).toBeVisible()

    // The layer item should be visible in the list
    await expect(page.locator('.layer-item')).toBeVisible()
  })

  test('should show layer editor after adding a layer', async ({ page }) => {
    await page.click('button:has-text("+ Add Layer")')

    // The layer editor section should appear
    await expect(page.locator('.selected-layer-editor')).toBeVisible()

    // Should have pattern selector, position inputs, blend mode, opacity
    await expect(page.locator('.layer-editor select').first()).toBeVisible()
    await expect(page.locator('.layer-position-grid')).toBeVisible()
    await expect(page.locator('.layer-blend-grid')).toBeVisible()
    await expect(page.locator('.layer-editor label:has-text("Blend Mode")')).toBeVisible()
    await expect(page.locator('.layer-editor label:has-text("Opacity")')).toBeVisible()
  })

  test('should be able to add multiple layers', async ({ page }) => {
    await page.click('button:has-text("+ Add Layer")')
    await expect(page.locator('label:has-text("Layers (1)")')).toBeVisible()

    await page.click('button:has-text("+ Add Layer")')
    await expect(page.locator('label:has-text("Layers (2)")')).toBeVisible()

    await page.click('button:has-text("+ Add Layer")')
    await expect(page.locator('label:has-text("Layers (3)")')).toBeVisible()

    // Should have 3 layer items
    await expect(page.locator('.layer-item')).toHaveCount(3)
  })

  test('should remove a layer when clicking delete button', async ({ page }) => {
    // Add a layer
    await page.click('button:has-text("+ Add Layer")')
    await expect(page.locator('label:has-text("Layers (1)")')).toBeVisible()

    // Click the delete button (× character)
    await page.click('.layer-item .icon-btn.delete')

    // Should now show 0 layers
    await expect(page.locator('label:has-text("Layers (0)")')).toBeVisible()
  })

  test('should select a layer when clicking on it', async ({ page }) => {
    // Add two layers
    await page.click('button:has-text("+ Add Layer")')
    await page.click('button:has-text("+ Add Layer")')

    // Both layer items should exist
    const layerItems = page.locator('.layer-item')
    await expect(layerItems).toHaveCount(2)

    // Click on the first layer
    await layerItems.first().click()

    // First layer should be selected
    await expect(layerItems.first()).toHaveClass(/selected/)
  })

  test('should update JSON when adding layers', async ({ page }) => {
    // Open the advanced panel
    await page.click('summary:has-text("Advanced")')

    // Get the textarea
    const textarea = page.locator('.json-editor textarea')

    // Initially should have empty layers array
    const initialJson = await textarea.inputValue()
    expect(JSON.parse(initialJson).layers).toHaveLength(0)

    // Add a layer
    await page.click('button:has-text("+ Add Layer")')

    // JSON should now have 1 layer
    const updatedJson = await textarea.inputValue()
    expect(JSON.parse(updatedJson).layers).toHaveLength(1)
  })

  test('should enable print button after adding a layer', async ({ page }) => {
    // Print button should be disabled initially
    const printButton = page.locator('button:has-text("Print")')
    await expect(printButton).toBeDisabled()

    // Add a layer
    await page.click('button:has-text("+ Add Layer")')

    // Print button should now be enabled
    await expect(printButton).toBeEnabled()
  })

  test('should show preview after adding a layer', async ({ page }) => {
    // Preview should show placeholder text initially
    await expect(page.locator('.preview-placeholder-text')).toBeVisible()

    // Add a layer
    await page.click('button:has-text("+ Add Layer")')

    // Wait for preview to load (the image should appear)
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 10000 })
  })
})
