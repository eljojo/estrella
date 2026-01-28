import { signal, effect, computed } from '@preact/signals'
import { useEffect } from 'preact/hooks'
import { fetchJsonPreview, fetchCanvasLayout, printJson } from '../api'
import type { OverlayLayer } from './LayerCanvas'
import {
  ComponentEditor,
  COMPONENT_TYPES,
  createDefaultComponent,
  getComponentLabel,
  getComponentSummary,
  ensurePatternsFetched,
} from './ComponentEditor'
import DEFAULT_JSON from '../../../src/fixtures/canvas-showcase.json?raw'

const defaultDoc = JSON.parse(DEFAULT_JSON)

// Document state
const editorComponents = signal<any[]>(defaultDoc.document ?? [])
const editorSelectedIndex = signal<number | null>(null)

// Print options
export const cut = signal(true)

// UI state
const status = signal<{ type: 'success' | 'error'; message: string } | null>(null)
export const loading = signal(false)
const showAdvanced = signal(false)
const jsonError = signal<string | null>(null)

// Exports for App.tsx
export const editorPreviewUrl = signal<string>('')
export const editorCustomized = computed(() => editorComponents.value.length > 0)
export const editorCanPrint = computed(() => editorComponents.value.length > 0 && !loading.value)

// Canvas overlay state (for interactive element manipulation)
export const editorCanvasOverlay = signal<{
  layers: OverlayLayer[]
  canvasWidth: number
  canvasHeight: number
  yOffset: number
  documentHeight: number
} | null>(null)
export const editorCanvasElementIndex = signal<number | null>(null)

// Build document JSON from current state
function buildDocumentJson(): string {
  return JSON.stringify({
    document: editorComponents.value,
    cut: cut.value,
  })
}

// Throttled preview refresh (fires every 500ms during continuous changes, not just after idle)
let previewTimeout: number | null = null
let lastPreviewTime = 0
const PREVIEW_THROTTLE = 500

effect(() => {
  const components = editorComponents.value
  void cut.value

  if (previewTimeout) clearTimeout(previewTimeout)

  if (components.length === 0) {
    editorPreviewUrl.value = ''
    return
  }

  const elapsed = Date.now() - lastPreviewTime
  const delay = elapsed >= PREVIEW_THROTTLE ? 0 : PREVIEW_THROTTLE - elapsed

  previewTimeout = window.setTimeout(async () => {
    lastPreviewTime = Date.now()
    try {
      const url = await fetchJsonPreview(buildDocumentJson())
      const prev = editorPreviewUrl.value
      if (prev && prev.startsWith('blob:')) URL.revokeObjectURL(prev)
      editorPreviewUrl.value = url
    } catch (err) {
      console.error('Preview error:', err)
    }
  }, delay)
})

// Throttled canvas overlay layout fetch (fires every 500ms during continuous changes)
let layoutTimeout: number | null = null
let lastLayoutTime = 0

effect(() => {
  const components = editorComponents.value
  const selectedIdx = editorSelectedIndex.value
  const cutValue = cut.value

  if (layoutTimeout) clearTimeout(layoutTimeout)

  if (selectedIdx === null || !components[selectedIdx] || components[selectedIdx].type !== 'canvas') {
    editorCanvasOverlay.value = null
    editorCanvasElementIndex.value = null
    return
  }

  const elapsed = Date.now() - lastLayoutTime
  const delay = elapsed >= PREVIEW_THROTTLE ? 0 : PREVIEW_THROTTLE - elapsed

  layoutTimeout = window.setTimeout(async () => {
    lastLayoutTime = Date.now()
    try {
      const layout = await fetchCanvasLayout(components, selectedIdx, cutValue)
      editorCanvasOverlay.value = {
        layers: layout.elements,
        canvasWidth: layout.width,
        canvasHeight: layout.height,
        yOffset: layout.y_offset,
        documentHeight: layout.document_height,
      }
    } catch {
      editorCanvasOverlay.value = null
    }
  }, delay)
})

// Canvas overlay handlers (exported for App.tsx)
export function handleCanvasOverlaySelect(index: number | null) {
  editorCanvasElementIndex.value = index
}

export function handleCanvasOverlayUpdate(elementIndex: number, updates: Partial<OverlayLayer>) {
  const selectedIdx = editorSelectedIndex.value
  if (selectedIdx === null) return

  const comp = editorComponents.value[selectedIdx]
  if (!comp || comp.type !== 'canvas') return

  const elements = [...(comp.elements || [])]
  const el = { ...elements[elementIndex] }
  if (!el) return

  // Update position
  if ('x' in updates || 'y' in updates) {
    el.position = {
      x: updates.x ?? el.position?.x ?? 0,
      y: updates.y ?? el.position?.y ?? 0,
    }
  }

  // Map width/height to element-specific size params
  if ('height' in updates) {
    switch (el.type) {
      case 'pattern':
      case 'canvas':
        el.height = updates.height
        break
    }
  }
  if ('width' in updates) {
    switch (el.type) {
      case 'image':
      case 'canvas':
        el.width = updates.width
        break
    }
  }

  elements[elementIndex] = el
  updateComponent(selectedIdx, { elements })
}

export function handleCanvasOverlayDoubleClick(elementIndex: number) {
  editorCanvasElementIndex.value = elementIndex
  // Focus content textarea after DOM update
  setTimeout(() => {
    const textarea = document.querySelector(
      '.selected-layer-editor .layer-editor .component-textarea'
    ) as HTMLTextAreaElement
    textarea?.focus()
  }, 50)
}

// Print handler
async function handlePrint() {
  if (editorComponents.value.length === 0) {
    status.value = { type: 'error', message: 'Add at least one component' }
    return
  }

  loading.value = true
  status.value = null

  try {
    const result = await printJson(buildDocumentJson())
    if (result.success) {
      status.value = { type: 'success', message: result.message || 'Printed!' }
    } else {
      status.value = { type: 'error', message: result.error || 'Print failed' }
    }
  } catch (err) {
    status.value = { type: 'error', message: `Error: ${err}` }
  } finally {
    loading.value = false
  }
}

export function triggerEditorPrint() {
  return handlePrint()
}

// Component operations
function addComponent(type: string) {
  const comp = createDefaultComponent(type)
  editorComponents.value = [...editorComponents.value, comp]
  editorSelectedIndex.value = editorComponents.value.length - 1
}

function removeComponent(index: number) {
  const newComps = editorComponents.value.filter((_, i) => i !== index)
  editorComponents.value = newComps

  const sel = editorSelectedIndex.value
  if (sel === index) {
    editorSelectedIndex.value = newComps.length > 0 ? Math.max(0, index - 1) : null
  } else if (sel !== null && sel > index) {
    editorSelectedIndex.value = sel - 1
  }
}

function moveComponent(index: number, direction: 'up' | 'down') {
  const newIndex = direction === 'up' ? index - 1 : index + 1
  if (newIndex < 0 || newIndex >= editorComponents.value.length) return

  const newComps = [...editorComponents.value]
  ;[newComps[index], newComps[newIndex]] = [newComps[newIndex], newComps[index]]
  editorComponents.value = newComps
  editorSelectedIndex.value = newIndex
}

function updateComponent(index: number, updates: any) {
  const newComps = [...editorComponents.value]
  newComps[index] = { ...newComps[index], ...updates }
  editorComponents.value = newComps
}

export function EditorForm() {
  const components = editorComponents.value
  const selectedIdx = editorSelectedIndex.value
  const selectedComponent = selectedIdx !== null ? components[selectedIdx] : null

  // Fetch patterns on mount (for pattern/canvas editors)
  useEffect(() => {
    ensurePatternsFetched()
  }, [])

  // JSON sync
  const handleJsonInput = (text: string) => {
    try {
      const doc = JSON.parse(text)
      if (!Array.isArray(doc.document)) {
        throw new Error('Missing "document" array')
      }
      editorComponents.value = doc.document
      if (doc.cut !== undefined) cut.value = doc.cut
      jsonError.value = null
    } catch (e) {
      jsonError.value = (e as Error).message
    }
  }

  const jsonText = JSON.stringify({ document: components, cut: cut.value }, null, 2)

  return (
    <div>
      {status.value && <div class={status.value.type}>{status.value.message}</div>}

      {/* Component List */}
      <div class="form-group">
        <label>Components ({components.length})</label>
        <div class="layers-list">
          {components.map((comp, index) => (
            <div
              key={index}
              class={`layer-item ${selectedIdx === index ? 'selected' : ''}`}
              onClick={() => (editorSelectedIndex.value = index)}
            >
              <span class="layer-name">
                {index + 1}. {getComponentLabel(comp.type)}
              </span>
              <span class="layer-meta">
                <span class="layer-blend">{getComponentSummary(comp)}</span>
              </span>
              <div class="layer-actions">
                <button
                  type="button"
                  class="icon-btn"
                  onClick={(e) => {
                    e.stopPropagation()
                    moveComponent(index, 'up')
                  }}
                  disabled={index === 0}
                  title="Move up"
                >
                  &uarr;
                </button>
                <button
                  type="button"
                  class="icon-btn"
                  onClick={(e) => {
                    e.stopPropagation()
                    moveComponent(index, 'down')
                  }}
                  disabled={index === components.length - 1}
                  title="Move down"
                >
                  &darr;
                </button>
                <button
                  type="button"
                  class="icon-btn delete"
                  onClick={(e) => {
                    e.stopPropagation()
                    removeComponent(index)
                  }}
                  title="Remove"
                >
                  &times;
                </button>
              </div>
            </div>
          ))}
          <select
            class="weave-add-select"
            value=""
            onChange={(e) => {
              const type = (e.target as HTMLSelectElement).value
              if (type) {
                addComponent(type)
                ;(e.target as HTMLSelectElement).value = ''
              }
            }}
          >
            <option value="">+ Add Component</option>
            {COMPONENT_TYPES.map((t) => (
              <option key={t.type} value={t.type}>
                {t.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Selected Component Editor */}
      {selectedComponent && selectedIdx !== null && (
        <div class="form-group selected-layer-editor">
          <label>
            Edit {getComponentLabel(selectedComponent.type)}: {getComponentSummary(selectedComponent)}
          </label>
          <ComponentEditor
            component={selectedComponent}
            onUpdate={(updates) => updateComponent(selectedIdx, updates)}
            canvasElementIndex={selectedComponent?.type === 'canvas' ? editorCanvasElementIndex.value : undefined}
            onCanvasElementSelect={selectedComponent?.type === 'canvas' ? (i) => { editorCanvasElementIndex.value = i } : undefined}
          />
        </div>
      )}

      {/* Advanced JSON Editor */}
      <details
        class="advanced-panel"
        open={showAdvanced.value}
        onToggle={(e) => (showAdvanced.value = (e.target as HTMLDetailsElement).open)}
      >
        <summary>Advanced (JSON)</summary>
        <div class="json-editor">
          {jsonError.value && <div class="error">{jsonError.value}</div>}
          <textarea
            value={jsonText}
            onInput={(e) => handleJsonInput((e.target as HTMLTextAreaElement).value)}
            rows={15}
            spellcheck={false}
          />
          <p class="hint">
            Edit JSON directly or paste a document to import. Use {'{{'}<em>name</em>{'}}'} in text
            with a top-level "variables" object.
          </p>
        </div>
      </details>
    </div>
  )
}
