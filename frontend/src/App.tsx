import { signal } from '@preact/signals'
import { Tabs } from './components/Tabs'
import { ReceiptForm, receiptPreviewUrl } from './components/ReceiptForm'
import { PatternForm, patternPreviewUrl } from './components/PatternForm'

export const activeTab = signal<'receipt' | 'patterns'>('receipt')

export function App() {
  const previewUrl = activeTab.value === 'receipt' ? receiptPreviewUrl.value : patternPreviewUrl.value

  return (
    <div class="container">
      <h1>Estrella Printer</h1>
      <p class="subtitle">Print text receipts or visual patterns to your thermal printer</p>
      <Tabs />
      <div class="main-layout">
        <div class="form-panel">
          {activeTab.value === 'receipt' ? <ReceiptForm /> : <PatternForm />}
        </div>
        <div class="preview-panel">
          <h3>Preview</h3>
          <div class="preview-container">
            {previewUrl ? (
              <img src={previewUrl} alt="Preview" />
            ) : (
              <div class="loading">
                {activeTab.value === 'receipt'
                  ? 'Start typing to see preview...'
                  : 'Select a pattern to see preview...'}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
