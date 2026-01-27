import { signal, effect, computed } from '@preact/signals'
import { fetchJsonPreview, printJson } from '../api'
import DEFAULT_JSON from '../../../src/fixtures/morning-briefing.json?raw'

const jsonText = signal(DEFAULT_JSON)
const status = signal<{ type: 'success' | 'error'; message: string } | null>(null)
const loading = signal(false)
const parseError = signal<string | null>(null)

export const jsonCustomized = computed(() => jsonText.value !== DEFAULT_JSON)

// Export preview URL for App.tsx
export const jsonPreviewUrl = signal('')

// Debounce timer for preview
let previewTimeout: number | null = null

// Update preview when JSON changes
effect(() => {
  const currentJson = jsonText.value

  if (previewTimeout) {
    clearTimeout(previewTimeout)
  }

  if (!currentJson.trim()) {
    jsonPreviewUrl.value = ''
    parseError.value = null
    return
  }

  // Validate JSON client-side first
  try {
    JSON.parse(currentJson)
    parseError.value = null
  } catch (err) {
    parseError.value = `JSON syntax error: ${(err as Error).message}`
    jsonPreviewUrl.value = ''
    return
  }

  // Debounce preview requests
  previewTimeout = window.setTimeout(async () => {
    try {
      const url = await fetchJsonPreview(currentJson)
      jsonPreviewUrl.value = url
      parseError.value = null
    } catch (err) {
      parseError.value = `${err}`
      jsonPreviewUrl.value = ''
    }
  }, 500)
})

export function JsonForm() {
  const handleSubmit = async (e: Event) => {
    e.preventDefault()

    if (!jsonText.value.trim()) {
      status.value = { type: 'error', message: 'JSON cannot be empty' }
      return
    }

    // Validate JSON
    try {
      JSON.parse(jsonText.value)
    } catch (err) {
      status.value = { type: 'error', message: `Invalid JSON: ${(err as Error).message}` }
      return
    }

    loading.value = true
    status.value = null

    try {
      const result = await printJson(jsonText.value)
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

  return (
    <form onSubmit={handleSubmit}>
      {status.value && <div class={status.value.type}>{status.value.message}</div>}

      <div class="form-group">
        <label for="json-body">JSON Document</label>
        <textarea
          id="json-body"
          spellcheck={false}
          value={jsonText.value}
          onInput={(e) => (jsonText.value = (e.target as HTMLTextAreaElement).value)}
        />
        {parseError.value && <p class="hint error-hint">{parseError.value}</p>}
        <p class="hint">
          Components: text, header, banner, line_item, total, divider, spacer, blank_line, columns,
          table, markdown, chart, qr_code, pdf417, barcode, pattern, nv_logo. Use {'{{'}<em>name</em>{'}}'}  in
          text with a top-level "variables" object. Built-ins: date, date_short, day, time,
          time_12h, datetime, year, iso_date.
        </p>
      </div>

      <button type="submit" disabled={loading.value}>
        {loading.value ? 'Printing...' : 'Print Document'}
      </button>
    </form>
  )
}
