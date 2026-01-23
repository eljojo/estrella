import { signal } from '@preact/signals'
import { uploadPhoto, buildPhotoPreviewUrl, printPhoto } from '../api'

// Photo session state
const sessionId = signal<string | null>(null)
const filename = signal('')
const rotation = signal<0 | 90 | 180 | 270>(0)
const dithering = signal<'jarvis' | 'atkinson' | 'bayer' | 'floyd-steinberg'>('jarvis')
const brightness = signal(0)
const contrast = signal(0)
const renderMode = signal<'raster' | 'band'>('raster')
const cut = signal(true)
const status = signal<{ type: 'success' | 'error'; message: string } | null>(null)
const uploading = signal(false)
const printing = signal(false)

// Preview URL - updated only when we actually want to fetch
export const photoPreviewUrl = signal('')
export const photoGrayscaleActive = signal(false)

// Request state for debouncing
let debounceTimer: ReturnType<typeof setTimeout> | null = null
let requestPending = false
let needsRefreshAfter = false
const DEBOUNCE_MS = 300
let grayscaleOnNextLoad = false

// Store image data for re-upload if session expires
let storedImageData: ArrayBuffer | null = null
let storedFilename: string = ''

// Build current preview URL from current state
function buildCurrentPreviewUrl(): string {
  if (!sessionId.value) return ''
  return buildPhotoPreviewUrl(
    sessionId.value,
    rotation.value,
    dithering.value,
    brightness.value,
    contrast.value,
    Date.now() // Cache bust
  )
}

// Actually trigger a preview update
function triggerPreviewUpdate() {
  if (!sessionId.value) return

  if (requestPending) {
    // Request in flight - mark that we need another when it's done
    needsRefreshAfter = true
    return
  }

  requestPending = true
  needsRefreshAfter = false

  const url = buildCurrentPreviewUrl()

  // Create an image to preload and detect when loading completes
  const img = new Image()
  img.onload = () => {
    requestPending = false
    photoPreviewUrl.value = url
    if (grayscaleOnNextLoad) {
      grayscaleOnNextLoad = false
      photoGrayscaleActive.value = false
      requestAnimationFrame(() => {
        photoGrayscaleActive.value = true
      })
    }

    // If settings changed while loading, trigger another update
    if (needsRefreshAfter) {
      needsRefreshAfter = false
      triggerPreviewUpdate()
    }
  }
  img.onerror = () => {
    requestPending = false
    // On error, still check if we need to refresh
    if (needsRefreshAfter) {
      needsRefreshAfter = false
      triggerPreviewUpdate()
    }
  }
  img.src = url
}

// Handle file selection
async function handleFileSelect(file: File) {
  if (!file) return

  status.value = null
  uploading.value = true

  try {
    const arrayBuffer = await file.arrayBuffer()
    storedImageData = arrayBuffer
    storedFilename = file.name

    const response = await uploadPhoto(arrayBuffer, file.name)
    sessionId.value = response.id
    filename.value = response.filename
    grayscaleOnNextLoad = true
    triggerPreviewUpdate()
  } catch (err) {
    status.value = { type: 'error', message: `Upload failed: ${err}` }
    sessionId.value = null
    photoGrayscaleActive.value = false
  } finally {
    uploading.value = false
  }
}

export function handlePhotoDrop(file: File) {
  void handleFileSelect(file)
}

// Re-upload if session expired
async function reuploadIfNeeded(): Promise<boolean> {
  if (!storedImageData || !storedFilename) return false

  try {
    const response = await uploadPhoto(storedImageData, storedFilename)
    sessionId.value = response.id
    return true
  } catch {
    return false
  }
}

export function PhotoForm() {
  const handleFileInput = (e: Event) => {
    const input = e.target as HTMLInputElement
    const file = input.files?.[0]
    if (file) handleFileSelect(file)
  }

  const handleDrop = (e: DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    const file = e.dataTransfer?.files[0]
    if (file && file.type.startsWith('image/')) {
      handleFileSelect(file)
    }
  }

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
  }

  const handleRotate = (direction: 'cw' | 'ccw') => {
    const current = rotation.value
    if (direction === 'cw') {
      rotation.value = ((current + 90) % 360) as 0 | 90 | 180 | 270
    } else {
      rotation.value = ((current - 90 + 360) % 360) as 0 | 90 | 180 | 270
    }
    handleSettingChangeImmediate()
  }

  const handlePrint = async () => {
    if (!sessionId.value) {
      status.value = { type: 'error', message: 'No image uploaded' }
      return
    }

    printing.value = true
    status.value = null

    try {
      const result = await printPhoto(
        sessionId.value,
        rotation.value,
        dithering.value,
        brightness.value,
        contrast.value,
        renderMode.value,
        cut.value
      )

      if (result.success) {
        status.value = { type: 'success', message: result.message || 'Printed successfully!' }
      } else {
        // Check if session expired
        if (result.error?.includes('not found') || result.error?.includes('expired')) {
          // Try to re-upload
          if (await reuploadIfNeeded()) {
            // Retry print
            const retryResult = await printPhoto(
              sessionId.value!,
              rotation.value,
              dithering.value,
              brightness.value,
              contrast.value,
              renderMode.value,
              cut.value
            )
            if (retryResult.success) {
              status.value = { type: 'success', message: retryResult.message || 'Printed successfully!' }
            } else {
              status.value = { type: 'error', message: retryResult.error || 'Print failed' }
            }
          } else {
            status.value = { type: 'error', message: 'Session expired. Please re-upload the image.' }
            sessionId.value = null
          }
        } else {
          status.value = { type: 'error', message: result.error || 'Print failed' }
        }
      }
    } catch (err) {
      status.value = { type: 'error', message: `Error: ${err}` }
    } finally {
      printing.value = false
    }
  }

  // Debounced preview update - waits for user to stop adjusting
  const handleSettingChange = () => {
    if (debounceTimer) {
      clearTimeout(debounceTimer)
    }
    debounceTimer = setTimeout(() => {
      debounceTimer = null
      triggerPreviewUpdate()
    }, DEBOUNCE_MS)
  }

  // Immediate preview update (for rotation, dithering changes)
  const handleSettingChangeImmediate = () => {
    if (debounceTimer) {
      clearTimeout(debounceTimer)
      debounceTimer = null
    }
    triggerPreviewUpdate()
  }

  const handleClear = () => {
    sessionId.value = null
    filename.value = ''
    rotation.value = 0
    brightness.value = 0
    contrast.value = 0
    storedImageData = null
    storedFilename = ''
    status.value = null
    photoPreviewUrl.value = ''
    requestPending = false
    needsRefreshAfter = false
    grayscaleOnNextLoad = false
    photoGrayscaleActive.value = false
  }

  return (
    <div class="drop-zone photo-drop-zone" onDrop={handleDrop} onDragOver={handleDragOver}>
      {status.value && <div class={status.value.type}>{status.value.message}</div>}

      <div class="form-group">
        <label>Image</label>
        <div class="file-input-wrapper">
          <input
            type="file"
            accept="image/*"
            onChange={handleFileInput}
            id="photo-input"
            class="hidden-file-input"
          />
          <label for="photo-input" class="file-input-label">
            {uploading.value
              ? 'Uploading...'
              : filename.value
                ? filename.value
                : 'Choose Image or Drag & Drop'}
          </label>
        </div>
        <p class="hint">Supports JPEG, PNG, GIF, WEBP</p>
      </div>

      {sessionId.value && (
        <>
          <div class="form-group">
            <label>Rotation</label>
            <div class="button-row rotation-buttons">
              <button
                type="button"
                class="button-secondary"
                onClick={() => handleRotate('ccw')}
                title="Rotate counter-clockwise"
              >
                Rotate Left
              </button>
              <span class="rotation-value">{rotation.value}</span>
              <button
                type="button"
                class="button-secondary"
                onClick={() => handleRotate('cw')}
                title="Rotate clockwise"
              >
                Rotate Right
              </button>
            </div>
          </div>

          <div class="form-group param-slider">
            <label for="brightness">
              Brightness
              <span class="param-value">{brightness.value}</span>
            </label>
            <input
              type="range"
              id="brightness"
              min="-100"
              max="100"
              value={brightness.value}
              onInput={(e) => {
                brightness.value = parseInt((e.target as HTMLInputElement).value)
                handleSettingChange()
              }}
            />
          </div>

          <div class="form-group param-slider">
            <label for="contrast">
              Contrast
              <span class="param-value">{contrast.value}</span>
            </label>
            <input
              type="range"
              id="contrast"
              min="-100"
              max="100"
              value={contrast.value}
              onInput={(e) => {
                contrast.value = parseInt((e.target as HTMLInputElement).value)
                handleSettingChange()
              }}
            />
          </div>

          <div class="form-group">
            <label for="dither">Dithering</label>
            <select
              id="dither"
              value={dithering.value}
              onChange={(e) => {
                dithering.value = (e.target as HTMLSelectElement).value as
                  | 'jarvis'
                  | 'atkinson'
                  | 'bayer'
                  | 'floyd-steinberg'
                handleSettingChangeImmediate()
              }}
            >
              <option value="jarvis">Jarvis (smooth)</option>
              <option value="atkinson">Atkinson (classic Mac)</option>
              <option value="bayer">Bayer (ordered)</option>
              <option value="floyd-steinberg">Floyd-Steinberg (diffusion)</option>
            </select>
          </div>

          <div class="form-group">
            <label for="mode">Render Mode</label>
            <select
              id="mode"
              value={renderMode.value}
              onChange={(e) => {
                renderMode.value = (e.target as HTMLSelectElement).value as 'raster' | 'band'
              }}
            >
              <option value="raster">Raster</option>
              <option value="band">Band (24-row chunks)</option>
            </select>
          </div>

          <div class="form-group checkbox-group">
            <label>
              <input
                type="checkbox"
                checked={cut.value}
                onChange={(e) => (cut.value = (e.target as HTMLInputElement).checked)}
              />
              Cut page after printing
            </label>
          </div>
        </>
      )}

      <div class="button-row">
        {sessionId.value && (
          <button type="button" class="button-secondary" onClick={handleClear}>
            Clear
          </button>
        )}
        <button type="button" onClick={handlePrint} disabled={!sessionId.value || printing.value}>
          {printing.value ? 'Printing...' : 'Print'}
        </button>
      </div>
    </div>
  )
}
