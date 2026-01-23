import { signal, computed, effect } from '@preact/signals'
import {
  fetchPatterns,
  fetchParams,
  fetchRandomParams,
  buildPreviewUrl,
  printPattern,
  ParamSpec,
} from '../api'

const patterns = signal<string[]>([])
const selectedPattern = signal('')
const params = signal<Record<string, string>>({})
const specs = signal<ParamSpec[]>([])
const lengthMm = signal(100)
const dithering = signal<'bayer' | 'floyd-steinberg' | 'atkinson' | 'jarvis'>('jarvis')
const renderMode = signal<'raster' | 'band'>('raster')
const cut = signal(true)
const printDetails = signal(true)
const status = signal<{ type: 'success' | 'error'; message: string } | null>(null)
const loading = signal(false)
const previewKey = signal(0) // Force refresh preview

// Export preview URL for App.tsx
export const patternPreviewUrl = computed(() => {
  if (!selectedPattern.value) return ''
  // Include previewKey to force refresh
  void previewKey.value
  return buildPreviewUrl(
    selectedPattern.value,
    lengthMm.value,
    params.value,
    dithering.value,
    renderMode.value
  )
})

// Fetch pattern list on mount
effect(() => {
  fetchPatterns()
    .then((p) => (patterns.value = p.sort()))
    .catch((e) => console.error('Failed to fetch patterns:', e))
})

// Fetch params when pattern changes
effect(() => {
  if (selectedPattern.value) {
    fetchParams(selectedPattern.value)
      .then((info) => {
        params.value = info.params
        specs.value = info.specs
      })
      .catch((e) => {
        console.error('Failed to fetch params:', e)
      })
  }
})

// Helper to render the appropriate input for a param spec
// Exported for use in WeaveForm
export function ParamInput({
  spec,
  value,
  onChange,
}: {
  spec: ParamSpec
  value: string
  onChange: (value: string) => void
}) {
  const id = `param-${spec.name}`

  // Slider type
  if (typeof spec.param_type === 'object' && 'slider' in spec.param_type) {
    const { min, max, step } = spec.param_type.slider
    return (
      <div class="param-item param-slider">
        <label for={id}>
          {spec.label}
          <span class="param-value">{value}</span>
        </label>
        <input
          type="range"
          id={id}
          min={min}
          max={max}
          step={step}
          value={value}
          onInput={(e) => onChange((e.target as HTMLInputElement).value)}
          title={spec.description}
        />
      </div>
    )
  }

  // Float type
  if (typeof spec.param_type === 'object' && 'float' in spec.param_type) {
    const { min, max, step } = spec.param_type.float
    return (
      <div class="param-item">
        <label for={id}>{spec.label}</label>
        <input
          type="number"
          id={id}
          min={min ?? undefined}
          max={max ?? undefined}
          step={step ?? 0.01}
          value={value}
          onInput={(e) => onChange((e.target as HTMLInputElement).value)}
          title={spec.description}
        />
      </div>
    )
  }

  // Int type
  if (typeof spec.param_type === 'object' && 'int' in spec.param_type) {
    const { min, max } = spec.param_type.int
    return (
      <div class="param-item">
        <label for={id}>{spec.label}</label>
        <input
          type="number"
          id={id}
          min={min ?? undefined}
          max={max ?? undefined}
          step={1}
          value={value}
          onInput={(e) => onChange((e.target as HTMLInputElement).value)}
          title={spec.description}
        />
      </div>
    )
  }

  // Select type
  if (typeof spec.param_type === 'object' && 'select' in spec.param_type) {
    const { options } = spec.param_type.select
    return (
      <div class="param-item">
        <label for={id}>{spec.label}</label>
        <select
          id={id}
          value={value}
          onChange={(e) => onChange((e.target as HTMLSelectElement).value)}
          title={spec.description}
        >
          {options.map((opt) => (
            <option key={opt} value={opt}>
              {opt}
            </option>
          ))}
        </select>
      </div>
    )
  }

  // Bool type
  if (spec.param_type === 'bool') {
    return (
      <div class="param-item param-bool">
        <label>
          <input
            type="checkbox"
            id={id}
            checked={value === 'true'}
            onChange={(e) => onChange((e.target as HTMLInputElement).checked ? 'true' : 'false')}
            title={spec.description}
          />
          {spec.label}
        </label>
      </div>
    )
  }

  // Fallback: basic text input
  return (
    <div class="param-item">
      <label for={id}>{spec.label}</label>
      <input
        type="text"
        id={id}
        value={value}
        onInput={(e) => onChange((e.target as HTMLInputElement).value)}
        title={spec.description}
      />
    </div>
  )
}

export function PatternForm() {
  const handleRandomize = async () => {
    if (!selectedPattern.value) return
    status.value = null

    try {
      const info = await fetchRandomParams(selectedPattern.value)
      params.value = info.params
      specs.value = info.specs
      previewKey.value++ // Force preview refresh
    } catch (err) {
      status.value = { type: 'error', message: `Failed to randomize: ${err}` }
    }
  }

  const handlePrint = async () => {
    if (!selectedPattern.value) {
      status.value = { type: 'error', message: 'Please select a pattern' }
      return
    }

    loading.value = true
    status.value = null

    try {
      const result = await printPattern(
        selectedPattern.value,
        lengthMm.value,
        params.value,
        dithering.value,
        renderMode.value,
        cut.value,
        printDetails.value
      )
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
  }

  const handleParamChange = (name: string, value: string) => {
    params.value = { ...params.value, [name]: value }
    previewKey.value++ // Force preview refresh
  }

  const handleSettingChange = () => {
    previewKey.value++ // Force preview refresh
  }

  // Build params UI: use specs if available, otherwise fall back to basic inputs
  const paramEntries = Object.entries(params.value)
  const hasSpecs = specs.value.length > 0

  return (
    <div>
      {status.value && <div class={status.value.type}>{status.value.message}</div>}

      <div class="form-group">
        <label for="pattern">Pattern</label>
        <select
          id="pattern"
          value={selectedPattern.value}
          onChange={(e) => (selectedPattern.value = (e.target as HTMLSelectElement).value)}
        >
          <option value="">Select a pattern...</option>
          {patterns.value.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
      </div>

      <div class="form-group">
        <label for="length">Length (mm)</label>
        <input
          type="number"
          id="length"
          min="10"
          max="500"
          value={lengthMm.value}
          onInput={(e) => {
            lengthMm.value = parseInt((e.target as HTMLInputElement).value) || 50
            handleSettingChange()
          }}
        />
        <p class="hint">Pattern height in millimeters (10-500mm)</p>
      </div>

      <div class="form-group">
        <label for="dither">Dithering</label>
        <select
          id="dither"
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

      <div class="form-group">
        <label for="mode">Render Mode</label>
        <select
          id="mode"
          value={renderMode.value}
          onChange={(e) => {
            renderMode.value = (e.target as HTMLSelectElement).value as 'raster' | 'band'
            handleSettingChange()
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
        <label>
          <input
            type="checkbox"
            checked={printDetails.value}
            onChange={(e) => (printDetails.value = (e.target as HTMLInputElement).checked)}
          />
          Print details (title and parameters)
        </label>
      </div>

      {paramEntries.length > 0 && (
        <div class="form-group">
          <label>Pattern Parameters</label>
          <div class="params-grid">
            {hasSpecs
              ? specs.value.map((spec) => (
                  <ParamInput
                    key={spec.name}
                    spec={spec}
                    value={params.value[spec.name] || ''}
                    onChange={(v) => handleParamChange(spec.name, v)}
                  />
                ))
              : paramEntries.map(([name, value]) => (
                  <div key={name} class="param-item">
                    <label for={`param-${name}`}>{name}</label>
                    <input
                      type="text"
                      id={`param-${name}`}
                      value={value}
                      onInput={(e) => handleParamChange(name, (e.target as HTMLInputElement).value)}
                    />
                  </div>
                ))}
          </div>
        </div>
      )}

      <div class="button-row">
        <button
          type="button"
          class="button-secondary"
          onClick={handleRandomize}
          disabled={!selectedPattern.value}
        >
          Randomize
        </button>
        <button type="button" onClick={handlePrint} disabled={!selectedPattern.value || loading.value}>
          {loading.value ? 'Printing...' : 'Print'}
        </button>
      </div>
    </div>
  )
}
