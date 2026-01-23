import { signal, effect } from '@preact/signals'
import { printReceipt, fetchReceiptPreview } from '../api'

const title = signal('Churra Mart')
const body = signal(`# Groceries

you can use **bold text** or \`  this thing  \` aaaaa!!

[underscoring]() is also possible

- thing one
- thing two

1. uno
1. dos
1. tres

#### gracias`)
export const cut = signal(true)
export const printDetails = signal(true)
const status = signal<{ type: 'success' | 'error'; message: string } | null>(null)
const loading = signal(false)

// Export preview URL for App.tsx
export const receiptPreviewUrl = signal('')

// Debounce timer for preview
let previewTimeout: number | null = null

// Update preview when title or body changes
effect(() => {
  const currentTitle = title.value
  const currentBody = body.value
  const currentCut = cut.value
  const currentPrintDetails = printDetails.value

  // Clear existing timeout
  if (previewTimeout) {
    clearTimeout(previewTimeout)
  }

  // Don't preview if body is empty
  if (!currentBody.trim()) {
    receiptPreviewUrl.value = ''
    return
  }

  // Debounce preview requests
  previewTimeout = window.setTimeout(async () => {
    try {
      const url = await fetchReceiptPreview(
        currentTitle,
        currentBody,
        currentCut,
        currentPrintDetails
      )
      receiptPreviewUrl.value = url
    } catch (err) {
      console.error('Preview failed:', err)
    }
  }, 500)
})

export function ReceiptForm() {
  const handleSubmit = async (e: Event) => {
    e.preventDefault()
    if (!body.value.trim()) {
      status.value = { type: 'error', message: 'Body cannot be empty' }
      return
    }

    loading.value = true
    status.value = null

    try {
      const result = await printReceipt(title.value, body.value, cut.value, printDetails.value)
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
      {status.value && (
        <div class={status.value.type}>{status.value.message}</div>
      )}

      <div class="form-group">
        <label for="title">Title (optional)</label>
        <input
          type="text"
          id="title"
          placeholder="Receipt Title"
          value={title.value}
          onInput={(e) => (title.value = (e.target as HTMLInputElement).value)}
        />
        <p class="hint">Optional header text for your receipt</p>
      </div>

      <div class="form-group">
        <label for="body">Body *</label>
        <textarea
          id="body"
          required
          placeholder="Enter your receipt text here...

Supports **Markdown** formatting:
- **Bold text**
- *Italic text*
- Lists and more!"
          value={body.value}
          onInput={(e) => (body.value = (e.target as HTMLTextAreaElement).value)}
        />
        <p class="hint">Required. Supports Markdown formatting.</p>
      </div>

      <button type="submit" disabled={loading.value}>
        {loading.value ? 'Printing...' : 'Print Receipt'}
      </button>
    </form>
  )
}
