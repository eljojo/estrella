import { signal, computed, effect } from '@preact/signals'
import {
  fetchPatterns,
  fetchParams,
  fetchRandomParams,
  fetchWeavePreview,
  printWeave,
  ParamSpec,
  WeavePatternEntry as ApiWeavePatternEntry,
} from '../api'
import { ParamInput } from './PatternForm'

// Curves hardcoded (matches BlendCurve in src/render/weave.rs)
const BLEND_CURVES = ['linear', 'smooth', 'ease-in', 'ease-out'] as const

// Internal weave pattern entry with UI state
interface WeavePatternEntry {
  id: string
  name: string
  params: Record<string, string>
  specs: ParamSpec[]
  collapsed: boolean
}

// State
const availablePatterns = signal<string[]>([])
const weavePatterns = signal<WeavePatternEntry[]>([])
const weaveLengthMm = signal(200)
const crossfadeMm = signal(30)
const blendCurve = signal<string>('smooth')
const dithering = signal<'bayer' | 'floyd-steinberg' | 'atkinson' | 'jarvis'>('jarvis')
const renderMode = signal<'raster' | 'band'>('raster')
const cut = signal(true)
const printDetails = signal(true)
const status = signal<{ type: 'success' | 'error'; message: string } | null>(null)
const loading = signal(false)
const previewLoading = signal(false)
const previewKey = signal(0)

// For drag and drop
const dragIndex = signal<number | null>(null)
const dragOverIndex = signal<number | null>(null)

// Generate unique IDs
let nextId = 0
const generateId = () => `weave-pattern-${nextId++}`

// Export preview URL for App.tsx (computed as blob URL)
export const weavePreviewUrl = signal<string>('')

// Debounced preview fetching
let previewTimeout: number | null = null

effect(() => {
  // Dependencies
  const patterns = weavePatterns.value
  const length = weaveLengthMm.value
  const crossfade = crossfadeMm.value
  const curve = blendCurve.value
  const dither = dithering.value
  const mode = renderMode.value
  void previewKey.value // Force refresh dependency

  if (previewTimeout) clearTimeout(previewTimeout)

  if (patterns.length < 2) {
    weavePreviewUrl.value = ''
    return
  }

  previewLoading.value = true

  previewTimeout = window.setTimeout(async () => {
    try {
      const apiPatterns: ApiWeavePatternEntry[] = patterns.map((p) => ({
        name: p.name,
        params: p.params,
      }))
      const url = await fetchWeavePreview(apiPatterns, length, crossfade, curve, dither, mode)
      weavePreviewUrl.value = url
    } catch (err) {
      console.error('Weave preview failed:', err)
      weavePreviewUrl.value = ''
    } finally {
      previewLoading.value = false
    }
  }, 500)
})

// Fetch available patterns on mount
effect(() => {
  fetchPatterns()
    .then((p) => (availablePatterns.value = p.sort()))
    .catch((e) => console.error('Failed to fetch patterns:', e))
})

// Add a pattern to the weave
async function addPattern(name: string) {
  if (!name) return

  try {
    const info = await fetchParams(name)
    const entry: WeavePatternEntry = {
      id: generateId(),
      name: info.name,
      params: info.params,
      specs: info.specs,
      collapsed: false,
    }
    weavePatterns.value = [...weavePatterns.value, entry]
    previewKey.value++
  } catch (err) {
    status.value = { type: 'error', message: `Failed to add pattern: ${err}` }
  }
}

// Remove a pattern from the weave
function removePattern(id: string) {
  weavePatterns.value = weavePatterns.value.filter((p) => p.id !== id)
  previewKey.value++
}

// Toggle collapsed state
function toggleCollapsed(id: string) {
  weavePatterns.value = weavePatterns.value.map((p) =>
    p.id === id ? { ...p, collapsed: !p.collapsed } : p
  )
}

// Update a pattern's param
function updatePatternParam(id: string, paramName: string, value: string) {
  weavePatterns.value = weavePatterns.value.map((p) =>
    p.id === id ? { ...p, params: { ...p.params, [paramName]: value } } : p
  )
  previewKey.value++
}

// Randomize a single pattern
async function randomizePattern(id: string, name: string) {
  try {
    const info = await fetchRandomParams(name)
    weavePatterns.value = weavePatterns.value.map((p) =>
      p.id === id ? { ...p, params: info.params, specs: info.specs } : p
    )
    previewKey.value++
  } catch (err) {
    status.value = { type: 'error', message: `Failed to randomize: ${err}` }
  }
}

// Randomize all patterns
async function randomizeAll() {
  for (const pattern of weavePatterns.value) {
    await randomizePattern(pattern.id, pattern.name)
  }
}

// Drag and drop handlers
function handleDragStart(index: number) {
  dragIndex.value = index
}

function handleDragOver(e: DragEvent, index: number) {
  e.preventDefault()
  dragOverIndex.value = index
}

function handleDragEnd() {
  dragIndex.value = null
  dragOverIndex.value = null
}

function handleDrop(e: DragEvent, dropIndex: number) {
  e.preventDefault()
  const fromIndex = dragIndex.value
  if (fromIndex === null || fromIndex === dropIndex) {
    handleDragEnd()
    return
  }

  const patterns = [...weavePatterns.value]
  const [removed] = patterns.splice(fromIndex, 1)
  patterns.splice(dropIndex, 0, removed)
  weavePatterns.value = patterns
  previewKey.value++
  handleDragEnd()
}

// Print the weave
async function handlePrint() {
  if (weavePatterns.value.length < 2) {
    status.value = { type: 'error', message: 'Add at least 2 patterns to print' }
    return
  }

  loading.value = true
  status.value = null

  try {
    const apiPatterns: ApiWeavePatternEntry[] = weavePatterns.value.map((p) => ({
      name: p.name,
      params: p.params,
    }))
    const result = await printWeave(
      apiPatterns,
      weaveLengthMm.value,
      crossfadeMm.value,
      blendCurve.value,
      dithering.value,
      renderMode.value,
      cut.value,
      printDetails.value
    )
    if (result.success) {
      status.value = { type: 'success', message: result.message || 'Weave printed!' }
    } else {
      status.value = { type: 'error', message: result.error || 'Print failed' }
    }
  } catch (err) {
    status.value = { type: 'error', message: `Error: ${err}` }
  } finally {
    loading.value = false
  }
}

function handleSettingChange() {
  previewKey.value++
}

export function WeaveForm() {
  return (
    <div>
      {status.value && <div class={status.value.type}>{status.value.message}</div>}

      {/* Weave Settings */}
      <div class="form-group">
        <label for="weave-length">Total Length (mm)</label>
        <input
          type="number"
          id="weave-length"
          min="50"
          max="1000"
          value={weaveLengthMm.value}
          onInput={(e) => {
            weaveLengthMm.value = parseInt((e.target as HTMLInputElement).value) || 200
            handleSettingChange()
          }}
        />
        <p class="hint">Total weave height (50-1000mm)</p>
      </div>

      <div class="form-group">
        <label for="crossfade">Crossfade (mm)</label>
        <input
          type="number"
          id="crossfade"
          min="5"
          max="100"
          value={crossfadeMm.value}
          onInput={(e) => {
            crossfadeMm.value = parseInt((e.target as HTMLInputElement).value) || 30
            handleSettingChange()
          }}
        />
        <p class="hint">Transition length between patterns (5-100mm)</p>
      </div>

      <div class="form-group">
        <label for="curve">Blend Curve</label>
        <select
          id="curve"
          value={blendCurve.value}
          onChange={(e) => {
            blendCurve.value = (e.target as HTMLSelectElement).value
            handleSettingChange()
          }}
        >
          {BLEND_CURVES.map((c) => (
            <option key={c} value={c}>
              {c.charAt(0).toUpperCase() + c.slice(1)}
            </option>
          ))}
        </select>
      </div>

      <div class="form-group">
        <label for="weave-dither">Dithering</label>
        <select
          id="weave-dither"
          value={dithering.value}
          onChange={(e) => {
            dithering.value = (e.target as HTMLSelectElement).value as 'bayer' | 'floyd-steinberg' | 'atkinson' | 'jarvis'
            handleSettingChange()
          }}
        >
          <option value="jarvis">Jarvis (smooth)</option>
          <option value="atkinson">Atkinson (classic Mac)</option>
          <option value="bayer">Bayer (ordered)</option>
          <option value="floyd-steinberg">Floyd-Steinberg (diffusion)</option>
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
        <label>
          <input
            type="checkbox"
            checked={printDetails.value}
            onChange={(e) => (printDetails.value = (e.target as HTMLInputElement).checked)}
          />
          Print details (title and parameters)
        </label>
      </div>

      {/* Pattern List */}
      <div class="form-group">
        <label>Patterns</label>
        <p class="hint">Add at least 2 patterns. Drag to reorder.</p>

        <div class="weave-list">
          {weavePatterns.value.map((entry, index) => (
            <div
              key={entry.id}
              class={`weave-entry ${entry.collapsed ? 'collapsed' : ''} ${
                dragOverIndex.value === index ? 'drag-over' : ''
              }`}
              draggable
              onDragStart={() => handleDragStart(index)}
              onDragOver={(e) => handleDragOver(e, index)}
              onDragEnd={handleDragEnd}
              onDrop={(e) => handleDrop(e, index)}
            >
              <div class="weave-entry-header" onClick={() => toggleCollapsed(entry.id)}>
                <span class="weave-entry-handle">&#x2261;</span>
                <span class="weave-entry-number">{index + 1}.</span>
                <span class="weave-entry-name">{entry.name}</span>
                <div class="weave-entry-actions">
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation()
                      randomizePattern(entry.id, entry.name)
                    }}
                    title="Randomize"
                  >
                    Rand
                  </button>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation()
                      removePattern(entry.id)
                    }}
                    title="Remove"
                  >
                    &times;
                  </button>
                </div>
                <span class="weave-entry-toggle">{entry.collapsed ? '+' : '-'}</span>
              </div>

              {!entry.collapsed && entry.specs.length > 0 && (
                <div class="weave-entry-params">
                  <div class="params-grid">
                    {entry.specs.map((spec) => (
                      <ParamInput
                        key={`${entry.id}-${spec.name}`}
                        spec={spec}
                        value={entry.params[spec.name] || ''}
                        onChange={(v) => updatePatternParam(entry.id, spec.name, v)}
                      />
                    ))}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>

        {/* Add Pattern */}
        <select
          class="weave-add-select"
          value=""
          onChange={(e) => {
            const val = (e.target as HTMLSelectElement).value
            if (val) {
              addPattern(val)
              ;(e.target as HTMLSelectElement).value = ''
            }
          }}
        >
          <option value="">+ Add Pattern...</option>
          {availablePatterns.value.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
      </div>

      {/* Actions */}
      <div class="button-row">
        <button
          type="button"
          class="button-secondary"
          onClick={randomizeAll}
          disabled={weavePatterns.value.length === 0}
        >
          Randomize All
        </button>
        <button
          type="button"
          onClick={handlePrint}
          disabled={weavePatterns.value.length < 2 || loading.value}
        >
          {loading.value ? 'Printing...' : 'Print'}
        </button>
      </div>

      {previewLoading.value && <p class="hint">Generating preview...</p>}
    </div>
  )
}
