import { test, expect, Page, Locator } from '@playwright/test'

/**
 * Select the canvas component in the Editor tab and wait for the overlay to appear.
 * The default fixture (canvas-showcase.json) has 3 document components;
 * the canvas is at index 2 with 4 inner elements.
 */
async function selectCanvasComponent(page: Page) {
  await page.goto('/')
  await page.click('button:has-text("Editor")')

  // Wait for preview to load (default fixture has components)
  await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })

  // Select the canvas component (3rd item in the list)
  const items = page.locator('.layer-item')
  await expect(items).toHaveCount(3)
  await items.nth(2).click()
  await expect(items.nth(2)).toHaveClass(/selected/)

  // Wait for the canvas overlay SVG to appear
  const overlay = page.locator('.layer-overlay')
  await expect(overlay).toBeAttached({ timeout: 10000 })
  return overlay
}

/**
 * Hover over the overlay SVG to make layer boxes render.
 * The overlay only shows children when the mouse is over it.
 */
async function hoverOverlay(page: Page, overlay: Locator) {
  const bounds = await overlay.boundingBox()
  expect(bounds).toBeTruthy()
  await page.mouse.move(bounds!.x + bounds!.width / 2, bounds!.y + bounds!.height / 2)
  // Wait for hover state to propagate and boxes to render
  const boxes = overlay.locator('.layer-box')
  await expect(boxes.first()).toBeAttached({ timeout: 3000 })
  return bounds!
}

test.describe('Canvas Overlay', () => {
  test('should show overlay boxes when hovering canvas preview', async ({ page }) => {
    const overlay = await selectCanvasComponent(page)
    await hoverOverlay(page, overlay)

    // The default canvas has 4 elements
    const boxes = overlay.locator('.layer-box')
    await expect(boxes).toHaveCount(4)
  })

  test('content bounds should be narrower than full canvas width', async ({ page }) => {
    const overlay = await selectCanvasComponent(page)
    await hoverOverlay(page, overlay)

    // The centered "Hello World" text (element index 1) should have tight content bounds
    const textBox = overlay.locator('.layer-box').nth(1)
    const width = Number(await textBox.getAttribute('width'))
    expect(width).toBeGreaterThan(0)
    expect(width).toBeLessThan(576)
  })

  test('overlay should show content offsets from backend', async ({ page }) => {
    const overlay = await selectCanvasComponent(page)
    await hoverOverlay(page, overlay)

    // The centered "Hello World" text should have x > 0 (left padding from centering)
    const textBox = overlay.locator('.layer-box').nth(1)
    const x = Number(await textBox.getAttribute('x'))
    expect(x).toBeGreaterThan(0)
  })
})

test.describe('Canvas Overlay - Drag', () => {
  test('should update element position on drag', async ({ page }) => {
    const overlay = await selectCanvasComponent(page)

    // Open JSON panel to observe state changes
    await page.click('summary:has-text("Advanced")')
    const textarea = page.locator('.json-editor textarea')

    // Read initial position of the flowfield (element index 3, topmost in SVG)
    const initialJson = JSON.parse(await textarea.inputValue())
    const initialPos = initialJson.document[2].elements[3].position

    // Hover to reveal boxes
    await hoverOverlay(page, overlay)

    // Drag the topmost element (index 3) — it receives mouse events without overlap issues
    const box = overlay.locator('.layer-box').nth(3)
    const boxBounds = await box.boundingBox()
    expect(boxBounds).toBeTruthy()

    const cx = boxBounds!.x + boxBounds!.width / 2
    const cy = boxBounds!.y + boxBounds!.height / 2
    await page.mouse.move(cx, cy)
    await page.mouse.down()
    await page.mouse.move(cx + 40, cy + 20, { steps: 5 })
    await page.mouse.up()

    // Wait for state to settle
    await page.waitForTimeout(1000)

    // Position should have changed in the JSON
    const afterJson = JSON.parse(await textarea.inputValue())
    const afterPos = afterJson.document[2].elements[3].position
    expect(afterPos).toBeTruthy()
    if (initialPos) {
      expect(afterPos.x).not.toBe(initialPos.x)
    }
  })
})

test.describe('Canvas Overlay - Resize', () => {
  test('should show resize handles on selected element', async ({ page }) => {
    const overlay = await selectCanvasComponent(page)
    await hoverOverlay(page, overlay)

    // Click on a box to select it (use force for overlapping SVG elements)
    await overlay.locator('.layer-box').first().click({ force: true })

    // Should show 4 resize handles (corners)
    const handles = overlay.locator('.resize-handle')
    await expect(handles).toHaveCount(4)
  })

  test('element height should not shrink on initial resize', async ({ page }) => {
    const overlay = await selectCanvasComponent(page)

    // Open JSON panel
    await page.click('summary:has-text("Advanced")')
    const textarea = page.locator('.json-editor textarea')

    // Read initial estrella height (element index 3, topmost in SVG)
    const initialJson = JSON.parse(await textarea.inputValue())
    const initialHeight = initialJson.document[2].elements[3].height
    expect(initialHeight).toBe(357)

    // Hover to reveal boxes
    await hoverOverlay(page, overlay)

    // Select estrella (index 3, topmost) — receives mouse events directly
    await overlay.locator('.layer-box').nth(3).click()

    // Wait for handles
    const handles = overlay.locator('.resize-handle')
    await expect(handles).toHaveCount(4)

    // Grab the SE handle (4th = index 3) and drag slightly down
    const seHandle = handles.nth(3)
    const hb = await seHandle.boundingBox()
    expect(hb).toBeTruthy()

    const hx = hb!.x + hb!.width / 2
    const hy = hb!.y + hb!.height / 2
    await page.mouse.move(hx, hy)
    await page.mouse.down()
    await page.mouse.move(hx, hy + 5, { steps: 3 })
    await page.mouse.up()

    // Wait for state to settle
    await page.waitForTimeout(500)

    // Height must NOT have shrunk
    const afterJson = JSON.parse(await textarea.inputValue())
    const afterHeight = afterJson.document[2].elements[3].height
    expect(afterHeight).toBeGreaterThanOrEqual(initialHeight)
  })

  test('resize should not produce negative height', async ({ page }) => {
    const overlay = await selectCanvasComponent(page)

    // Open JSON panel
    await page.click('summary:has-text("Advanced")')
    const textarea = page.locator('.json-editor textarea')

    // Hover to reveal boxes
    await hoverOverlay(page, overlay)

    // Select the flowfield pattern (index 3 — last element, topmost in SVG)
    await overlay.locator('.layer-box').nth(3).click()

    // Wait for handles
    const handles = overlay.locator('.resize-handle')
    await expect(handles).toHaveCount(4)

    // Grab SE handle and do a continuous drag (10 incremental moves)
    const seHandle = handles.nth(3)
    const hb = await seHandle.boundingBox()
    expect(hb).toBeTruthy()

    const hx = hb!.x + hb!.width / 2
    const hy = hb!.y + hb!.height / 2
    await page.mouse.move(hx, hy)
    await page.mouse.down()
    for (let i = 1; i <= 10; i++) {
      await page.mouse.move(hx, hy + 10 * i, { steps: 2 })
    }
    await page.mouse.up()

    await page.waitForTimeout(500)

    // Height must be positive (not the -45px bug)
    const afterJson = JSON.parse(await textarea.inputValue())
    const afterHeight = afterJson.document[2].elements[3].height
    expect(afterHeight).toBeGreaterThan(0)
  })
})
