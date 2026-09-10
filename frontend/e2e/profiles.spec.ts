import { test, expect, Page } from '@playwright/test'

async function getPreviewNaturalWidth(page: Page): Promise<number> {
  return page.locator('.preview-image').evaluate(
    (img: HTMLImageElement) => img.naturalWidth
  )
}

test.describe('Profiles', () => {
  // Tests share server-side profile state — run them sequentially
  test.describe.configure({ mode: 'serial' })

  // Reset to printer profile before each test
  test.beforeEach(async ({ request }) => {
    await request.put('/api/profiles/active', {
      data: { name: 'Star TSP650II' },
    })
  })

  test('profile selector visible and lists built-in profiles', async ({ page }) => {
    await page.goto('/')

    // Profile selector should be visible
    const selector = page.locator('.profile-selector select')
    await expect(selector).toBeVisible()

    // Should contain both built-in profiles
    const options = selector.locator('option')
    const texts = await options.allTextContents()
    expect(texts.some(t => t.includes('TSP650II'))).toBe(true)
    expect(texts.some(t => t.includes('Canvas'))).toBe(true)
  })

  test('Profile API returns correct data', async ({ request }) => {
    // GET /api/profiles should list built-in profiles
    const profilesRes = await request.get('/api/profiles')
    expect(profilesRes.ok()).toBe(true)
    const profiles = await profilesRes.json()
    expect(profiles.length).toBeGreaterThanOrEqual(2)

    // GET /api/profiles/active should return default (TSP650II)
    const activeRes = await request.get('/api/profiles/active')
    expect(activeRes.ok()).toBe(true)
    const active = await activeRes.json()
    expect(active.type).toBe('printer')
    expect(active.width).toBe(576)
  })

  test('switching to Canvas profile changes pattern preview dimensions', async ({ page }) => {
    await page.goto('/')
    await page.click('button:has-text("Patterns")')

    // Wait for preview to load with default TSP650II (576px)
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })
    const defaultWidth = await getPreviewNaturalWidth(page)
    // TSP650II: pattern preview renders at print width (576 dots)
    expect(defaultWidth).toBe(576)

    // Switch to Canvas profile via API
    const selector = page.locator('.profile-selector select')
    const options = await selector.locator('option').allTextContents()
    const canvasOption = options.find(t => t.includes('Canvas'))
    expect(canvasOption).toBeTruthy()
    await selector.selectOption({ label: canvasOption! })

    // Wait for preview to reload at new width
    await page.waitForTimeout(1000)
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })
    const canvasWidth = await getPreviewNaturalWidth(page)
    // Canvas profile renders at 1200px wide
    expect(canvasWidth).toBeGreaterThan(576)
  })

  test('Canvas profile hides Print, shows Download', async ({ page }) => {
    await page.goto('/')
    await page.click('button:has-text("Patterns")')
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })

    // With printer profile, Print should be visible
    await expect(page.locator('button:has-text("Print")')).toBeVisible()
    await expect(page.locator('.download-button')).not.toBeVisible()

    // Switch to Canvas profile
    const selector = page.locator('.profile-selector select')
    const options = await selector.locator('option').allTextContents()
    const canvasOption = options.find(t => t.includes('Canvas'))!
    await selector.selectOption({ label: canvasOption })
    await page.waitForTimeout(1000)
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })

    // Print should be hidden, Download should be visible
    await expect(page.locator('button:has-text("Print")')).not.toBeVisible()
    await expect(page.locator('.download-button')).toBeVisible()
  })

  test('switching back to printer profile restores Print button', async ({ page }) => {
    await page.goto('/')
    await page.click('button:has-text("Patterns")')
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })

    // Switch to Canvas
    const selector = page.locator('.profile-selector select')
    const options = await selector.locator('option').allTextContents()
    const canvasOption = options.find(t => t.includes('Canvas'))!
    const printerOption = options.find(t => t.includes('TSP650II'))!

    await selector.selectOption({ label: canvasOption })
    await page.waitForTimeout(500)
    await expect(page.locator('button:has-text("Print")')).not.toBeVisible()

    // Switch back to Printer
    await selector.selectOption({ label: printerOption })
    await page.waitForTimeout(500)
    await expect(page.locator('button:has-text("Print")')).toBeVisible()
  })

  test('Canvas profile works across tabs', async ({ page }) => {
    await page.goto('/')

    // Switch to Canvas profile
    const selector = page.locator('.profile-selector select')
    const options = await selector.locator('option').allTextContents()
    const canvasOption = options.find(t => t.includes('Canvas'))!
    await selector.selectOption({ label: canvasOption })
    await page.waitForTimeout(500)

    // Patterns tab: preview should load wider than 576
    await page.click('button:has-text("Patterns")')
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })
    const patternWidth = await getPreviewNaturalWidth(page)
    expect(patternWidth).toBeGreaterThan(576)

    // Weave tab: add 2 patterns, preview should load wider than 576
    await page.click('button:has-text("Weave")')
    await page.locator('select.weave-add-select').selectOption('ripple')
    await page.locator('select.weave-add-select').selectOption('waves')
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })
    const weaveWidth = await getPreviewNaturalWidth(page)
    expect(weaveWidth).toBeGreaterThan(576)
  })

  test('custom canvas dimensions via profile API', async ({ page, request }) => {
    // PUT a Canvas profile with width=800
    const putRes = await request.put('/api/profiles/active', {
      data: { type: 'canvas', name: 'Custom', width: 800, height: null },
    })
    expect(putRes.ok()).toBe(true)

    // Navigate to patterns tab and load a pattern
    await page.goto('/')
    await page.click('button:has-text("Patterns")')
    await expect(page.locator('.preview-image')).toBeVisible({ timeout: 15000 })

    // Verify preview renders at 800px wide
    const previewWidth = await getPreviewNaturalWidth(page)
    expect(previewWidth).toBe(800)
  })

  test('no console errors during profile switching', async ({ page }) => {
    const consoleErrors: string[] = []
    page.on('pageerror', err => consoleErrors.push(err.message))

    await page.goto('/')
    await page.click('button:has-text("Patterns")')
    await page.waitForTimeout(1000)

    const selector = page.locator('.profile-selector select')
    const options = await selector.locator('option').allTextContents()
    const canvasOption = options.find(t => t.includes('Canvas'))!
    const printerOption = options.find(t => t.includes('TSP650II'))!

    // Switch profiles multiple times
    await selector.selectOption({ label: canvasOption })
    await page.waitForTimeout(500)
    await selector.selectOption({ label: printerOption })
    await page.waitForTimeout(500)
    await selector.selectOption({ label: canvasOption })
    await page.waitForTimeout(500)

    // Navigate between tabs
    await page.click('button:has-text("Weave")')
    await page.waitForTimeout(500)
    await page.click('button:has-text("Receipt")')
    await page.waitForTimeout(500)

    expect(consoleErrors.length).toBe(0)
  })
})
