import { signal, effect, computed } from '@preact/signals'
import { fetchJsonPreview, printJson } from '../api'

const DEFAULT_JSON = JSON.stringify(
  {
    document: [
      { type: 'pattern', name: 'estrella', height: 160, params: { size: '0.7' } },
      { type: 'spacer', mm: 1 },
      { type: 'text', content: 'GOOD MORNING', center: true, bold: true, size: 2 },
      { type: 'text', content: 'Monday, January 27', center: true, font: 'B' },
      { type: 'divider', style: 'double' },

      { type: 'text', content: ' WEATHER ', bold: true, invert: true },
      { type: 'spacer', mm: 1 },
      { type: 'columns', left: 'Now', right: '6\u00b0C Cloudy' },
      { type: 'columns', left: 'High / Low', right: '11\u00b0C / 3\u00b0C' },
      { type: 'columns', left: 'Wind', right: '19 km/h NW' },
      { type: 'text', content: '\u26a0 Rain expected after 3pm', bold: true },

      { type: 'divider' },
      { type: 'text', content: ' BIRTHDAYS ', bold: true, invert: true },
      { type: 'spacer', mm: 1 },
      { type: 'text', content: '\ud83c\udf82 Ana turns 30 today!', scale: 2 },
      { type: 'text', content: 'Carlos on Wednesday', font: 'B' },

      { type: 'divider' },
      { type: 'text', content: ' TRASH ', bold: true, invert: true },
      { type: 'spacer', mm: 1 },
      { type: 'columns', left: 'Today', right: 'Recycling', bold: true, underline: true },
      { type: 'columns', left: 'Thursday', right: 'General waste' },

      { type: 'divider' },
      { type: 'text', content: ' CALENDAR ', bold: true, invert: true },
      { type: 'spacer', mm: 1 },
      { type: 'columns', left: '9:00', right: 'Standup' },
      { type: 'columns', left: '11:30', right: 'Dentist', bold: true },
      { type: 'columns', left: '14:00', right: 'Design review' },
      { type: 'columns', left: '18:30', right: 'Dinner w/ Alex', bold: true },

      { type: 'divider' },
      { type: 'text', content: ' NEWS ', bold: true, invert: true },
      { type: 'spacer', mm: 1 },
      {
        type: 'markdown',
        content:
          '- Scientists discover high-temp superconductor\n- City council approves new bike lanes\n- Local bakery wins national award',
      },

      { type: 'divider' },
      { type: 'text', content: ' GROCERIES ', bold: true, invert: true },
      { type: 'spacer', mm: 1 },
      { type: 'line_item', name: 'Oat milk', price: 2.49 },
      { type: 'line_item', name: 'Sourdough bread', price: 4.80 },
      { type: 'line_item', name: 'Avocados x3', price: 3.60 },
      { type: 'total', amount: 10.89 },

      { type: 'divider', style: 'double' },
      { type: 'text', content: 'Have a great day!', center: true, bold: true },
      { type: 'spacer', mm: 2 },
      { type: 'qr_code', data: 'https://calendar.google.com' },
      { type: 'spacer', mm: 2 },
    ],
    cut: true,
  },
  null,
  2
)

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
          Full component reference: text, header, line_item, total, divider, spacer, blank_line,
          columns, markdown, qr_code, pdf417, barcode, pattern, nv_logo
        </p>
      </div>

      <button type="submit" disabled={loading.value}>
        {loading.value ? 'Printing...' : 'Print Document'}
      </button>
    </form>
  )
}
