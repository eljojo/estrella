import { signal, effect } from '@preact/signals'
import { useEffect, useState, useCallback } from 'preact/hooks'
import {
  fetchPatterns,
  fetchComposerPatternParams,
  fetchComposerPreview,
  printComposer,
  BlendMode,
  ComposerLayer,
  ComposerSpec,
  ParamSpec,
} from '../api'
import { ParamInput } from './PatternForm'

// Printer resolution: 203 DPI = ~8 dots per mm
const DOTS_PER_MM = 8

// Canvas settings (module-level, rarely change)
const canvasHeightMm = signal(60) // Default 60mm
export const background = signal(0)
export const dithering = signal<'bayer' | 'floyd-steinberg' | 'atkinson' | 'jarvis'>('floyd-steinberg')
const renderMode = signal<'raster' | 'band'>('raster')

// Patterns list (fetched once)
const patterns = signal<string[]>([])

// Print options
export const cut = signal(true)

// UI state
export const status = signal<{ type: 'success' | 'error'; message: string } | null>(null)
export const loading = signal(false)
const layerSpecs = signal<Record<string, ParamSpec[]>>({})
const showAdvanced = signal(false)
const jsonError = signal<string | null>(null)

// Exports for App.tsx
export const composerPreviewUrl = signal<string>('')
export const composerCustomized = signal(false)
export const composerCanPrint = signal(false)

// Print function that can be called from App.tsx
let printHandler: (() => Promise<void>) | null = null
export function setComposerPrintHandler(handler: () => Promise<void>) {
  printHandler = handler
}
export async function triggerComposerPrint() {
  if (printHandler) {
    await printHandler()
  }
}

// Blend modes
const blendModes: BlendMode[] = ['normal', 'multiply', 'screen', 'overlay', 'add', 'difference', 'min', 'max']

// Flag to track if patterns have been fetched
let patternsFetched = false

// Fetch specs when a pattern is selected for a layer
const fetchLayerSpecs = async (patternName: string) => {
  if (layerSpecs.value[patternName]) return
  try {
    const info = await fetchComposerPatternParams(patternName)
    layerSpecs.value = { ...layerSpecs.value, [patternName]: info.specs }
  } catch (e) {
    console.error('Failed to fetch specs for', patternName, e)
  }
}

// Layer editor component
function LayerEditor({
  layer,
  index,
  onUpdate,
  onUpdateParam,
}: {
  layer: ComposerLayer
  index: number
  onUpdate: (index: number, updates: Partial<ComposerLayer>) => void
  onUpdateParam: (index: number, paramName: string, value: string) => void
}) {
  const specs = layerSpecs.value[layer.pattern] || []
  const patternList = patterns.value

  return (
    <div class="layer-editor">
      <div class="form-group">
        <label>Pattern</label>
        <select
          value={layer.pattern}
          onChange={(e) => {
            const pattern = (e.target as HTMLSelectElement).value
            onUpdate(index, { pattern, params: {} })
          }}
        >
          {patternList.length === 0 ? (
            <option value={layer.pattern}>{layer.pattern}</option>
          ) : (
            patternList.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))
          )}
        </select>
      </div>

      <div class="layer-position-grid">
        <div class="form-group">
          <label>X</label>
          <input
            type="number"
            value={layer.x}
            onInput={(e) => onUpdate(index, { x: parseInt((e.target as HTMLInputElement).value) || 0 })}
          />
        </div>
        <div class="form-group">
          <label>Y</label>
          <input
            type="number"
            value={layer.y}
            onInput={(e) => onUpdate(index, { y: parseInt((e.target as HTMLInputElement).value) || 0 })}
          />
        </div>
        <div class="form-group">
          <label>Width</label>
          <input
            type="number"
            min="1"
            value={layer.width}
            onInput={(e) => onUpdate(index, { width: parseInt((e.target as HTMLInputElement).value) || 1 })}
          />
        </div>
        <div class="form-group">
          <label>Height</label>
          <input
            type="number"
            min="1"
            value={layer.height}
            onInput={(e) => onUpdate(index, { height: parseInt((e.target as HTMLInputElement).value) || 1 })}
          />
        </div>
      </div>

      <div class="layer-blend-grid">
        <div class="form-group">
          <label>Blend Mode</label>
          <select
            value={layer.blend_mode}
            onChange={(e) => onUpdate(index, { blend_mode: (e.target as HTMLSelectElement).value as BlendMode })}
          >
            {blendModes.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </div>
        <div class="form-group">
          <label>Opacity</label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={layer.opacity}
            onInput={(e) => onUpdate(index, { opacity: parseFloat((e.target as HTMLInputElement).value) })}
          />
          <span class="opacity-value">{(layer.opacity * 100).toFixed(0)}%</span>
        </div>
      </div>

      {specs.length > 0 && (
        <div class="layer-params">
          <label>Pattern Parameters</label>
          <div class="params-grid">
            {specs.map((spec) => (
              <ParamInput
                key={spec.name}
                spec={spec}
                value={layer.params[spec.name] || ''}
                onChange={(v) => onUpdateParam(index, spec.name, v)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

export function ComposerForm() {
  // Use native useState for layers to guarantee re-renders
  const [layers, setLayers] = useState<ComposerLayer[]>([])
  const [selectedLayerIndex, setSelectedLayerIndex] = useState<number | null>(null)

  // Fetch patterns on mount
  useEffect(() => {
    if (!patternsFetched) {
      patternsFetched = true
      fetchPatterns()
        .then((p) => {
          if (Array.isArray(p)) {
            patterns.value = p.sort()
          }
        })
        .catch((e) => console.error('Failed to fetch patterns:', e))
    }
  }, [])

  // Build composition spec from state
  const canvasHeightPx = canvasHeightMm.value * DOTS_PER_MM
  const buildSpec = useCallback((): ComposerSpec => ({
    width: 576,
    height: canvasHeightPx,
    background: background.value,
    layers: layers,
  }), [layers, canvasHeightPx])

  // Debounced preview refresh
  useEffect(() => {
    let timeout: ReturnType<typeof setTimeout> | null = null
    let cancelled = false

    const refreshPreview = async () => {
      if (cancelled) return
      try {
        if (layers.length === 0) {
          composerPreviewUrl.value = ''
          composerCustomized.value = false
          return
        }
        composerCustomized.value = true
        const spec = buildSpec()
        const url = await fetchComposerPreview(spec, dithering.value)
        if (cancelled) return
        const currentUrl = composerPreviewUrl.value
        if (currentUrl && currentUrl.startsWith('blob:')) {
          URL.revokeObjectURL(currentUrl)
        }
        composerPreviewUrl.value = url
      } catch (err) {
        console.error('Preview error:', err)
      }
    }

    timeout = setTimeout(refreshPreview, 300)

    return () => {
      cancelled = true
      if (timeout) clearTimeout(timeout)
    }
  }, [layers, buildSpec])

  // Also refresh when canvas settings change
  useEffect(() => {
    const dispose = effect(() => {
      void canvasHeightMm.value
      void background.value
      void dithering.value
      // Trigger a re-render by updating a dummy state
      if (layers.length > 0) {
        setLayers(prev => [...prev])
      }
    })
    return dispose
  }, [layers.length])

  // Layer operations
  const addLayer = useCallback(() => {
    const patternName = patterns.value.length > 0 ? patterns.value[0] : 'ripple'
    const heightPx = canvasHeightMm.value * DOTS_PER_MM
    const newLayer: ComposerLayer = {
      pattern: patternName,
      params: {},
      x: 0,
      y: 0,
      width: 576,
      height: heightPx,
      blend_mode: 'normal',
      opacity: 1.0,
    }
    setLayers(prev => {
      const newLayers = [...prev, newLayer]
      setSelectedLayerIndex(newLayers.length - 1)
      return newLayers
    })
    fetchLayerSpecs(patternName)
  }, [])

  const removeLayer = useCallback((index: number) => {
    setLayers(prev => {
      const newLayers = prev.filter((_, i) => i !== index)
      setSelectedLayerIndex(current => {
        if (current === index) {
          return newLayers.length > 0 ? Math.max(0, index - 1) : null
        } else if (current !== null && current > index) {
          return current - 1
        }
        return current
      })
      return newLayers
    })
  }, [])

  const moveLayer = useCallback((index: number, direction: 'up' | 'down') => {
    setLayers(prev => {
      const newLayers = [...prev]
      const newIndex = direction === 'up' ? index - 1 : index + 1
      if (newIndex < 0 || newIndex >= newLayers.length) return prev
      ;[newLayers[index], newLayers[newIndex]] = [newLayers[newIndex], newLayers[index]]
      setSelectedLayerIndex(newIndex)
      return newLayers
    })
  }, [])

  const updateLayer = useCallback((index: number, updates: Partial<ComposerLayer>) => {
    setLayers(prev => {
      const newLayers = [...prev]
      newLayers[index] = { ...newLayers[index], ...updates }
      return newLayers
    })
    if (updates.pattern) {
      fetchLayerSpecs(updates.pattern)
    }
  }, [])

  const updateLayerParam = useCallback((index: number, paramName: string, value: string) => {
    setLayers(prev => {
      const newLayers = [...prev]
      newLayers[index] = {
        ...newLayers[index],
        params: { ...newLayers[index].params, [paramName]: value },
      }
      return newLayers
    })
  }, [])

  const handleJsonInput = useCallback((text: string) => {
    try {
      const spec: ComposerSpec = JSON.parse(text)
      if (typeof spec.width !== 'number' || typeof spec.height !== 'number') {
        throw new Error('Invalid spec: missing width or height')
      }
      if (!Array.isArray(spec.layers)) {
        throw new Error('Invalid spec: layers must be an array')
      }
      canvasHeightMm.value = Math.round(spec.height / DOTS_PER_MM)
      background.value = spec.background ?? 0
      setLayers(spec.layers)
      jsonError.value = null
    } catch (e) {
      jsonError.value = (e as Error).message
    }
  }, [])

  const handlePrint = useCallback(async () => {
    if (layers.length === 0) {
      status.value = { type: 'error', message: 'Add at least one layer' }
      return
    }

    loading.value = true
    status.value = null

    try {
      const result = await printComposer(buildSpec(), dithering.value, renderMode.value, cut.value)
      if (result.success) {
        status.value = { type: 'success', message: result.message || 'Printed successfully!' }
      } else {
        status.value = { type: 'error', message: result.error || 'Print failed' }
      }
    } catch (err) {
      status.value = { type: 'error', message: `Error: ${err}` }
    } finally {
      loading.value = false
    }
  }, [layers, buildSpec])

  // Update canPrint signal and register print handler
  useEffect(() => {
    composerCanPrint.value = layers.length > 0 && !loading.value
    setComposerPrintHandler(handlePrint)
    return () => setComposerPrintHandler(async () => {})
  }, [layers.length, handlePrint])

  const jsonSpec = JSON.stringify(buildSpec(), null, 2)
  const selectedLayer = selectedLayerIndex !== null ? layers[selectedLayerIndex] : null

  return (
    <div>
      {status.value && <div class={status.value.type}>{status.value.message}</div>}

      {/* Canvas Settings */}
      <div class="form-group">
        <label>Canvas Height (mm)</label>
        <input
          type="number"
          min="10"
          max="500"
          value={canvasHeightMm.value}
          onInput={(e) => (canvasHeightMm.value = parseInt((e.target as HTMLInputElement).value) || 60)}
        />
        <p class="hint">Width is fixed at 72mm (576 dots). Height: {canvasHeightMm.value * DOTS_PER_MM}px</p>
      </div>

      {/* Layers */}
      <div class="form-group">
        <label>Layers ({layers.length})</label>
        <div class="layers-list">
          {layers.map((layer, index) => (
            <div
              key={index}
              class={`layer-item ${selectedLayerIndex === index ? 'selected' : ''}`}
              onClick={() => setSelectedLayerIndex(index)}
            >
              <span class="layer-name">
                {index + 1}. {layer.pattern}
              </span>
              <span class="layer-meta">
                {layer.blend_mode !== 'normal' && <span class="layer-blend">{layer.blend_mode}</span>}
                {layer.opacity < 1 && <span class="layer-opacity">{(layer.opacity * 100).toFixed(0)}%</span>}
              </span>
              <div class="layer-actions">
                <button
                  type="button"
                  class="icon-btn"
                  onClick={(e) => {
                    e.stopPropagation()
                    moveLayer(index, 'up')
                  }}
                  disabled={index === 0}
                  title="Move up"
                >
                  ↑
                </button>
                <button
                  type="button"
                  class="icon-btn"
                  onClick={(e) => {
                    e.stopPropagation()
                    moveLayer(index, 'down')
                  }}
                  disabled={index === layers.length - 1}
                  title="Move down"
                >
                  ↓
                </button>
                <button
                  type="button"
                  class="icon-btn delete"
                  onClick={(e) => {
                    e.stopPropagation()
                    removeLayer(index)
                  }}
                  title="Remove"
                >
                  ×
                </button>
              </div>
            </div>
          ))}
          <button type="button" class="add-layer-btn" onClick={addLayer}>
            + Add Layer
          </button>
        </div>
      </div>

      {/* Selected Layer Editor */}
      {selectedLayer && selectedLayerIndex !== null && (
        <div class="form-group selected-layer-editor">
          <label>
            Edit Layer {selectedLayerIndex + 1}: {selectedLayer.pattern}
          </label>
          <LayerEditor
            layer={selectedLayer}
            index={selectedLayerIndex}
            onUpdate={updateLayer}
            onUpdateParam={updateLayerParam}
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
            value={jsonSpec}
            onInput={(e) => handleJsonInput((e.target as HTMLTextAreaElement).value)}
            rows={15}
            spellcheck={false}
          />
          <p class="hint">Edit JSON directly or copy to backup your composition</p>
        </div>
      </details>

      {/* Render Mode */}
      <div class="form-group">
        <label>Render Mode</label>
        <select
          value={renderMode.value}
          onChange={(e) => (renderMode.value = (e.target as HTMLSelectElement).value as 'raster' | 'band')}
        >
          <option value="raster">Raster</option>
          <option value="band">Band (24-row chunks)</option>
        </select>
      </div>
    </div>
  )
}
