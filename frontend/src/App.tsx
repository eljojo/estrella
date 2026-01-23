import { signal } from '@preact/signals'
import { Tabs } from './components/Tabs'
import { ReceiptForm, receiptPreviewUrl } from './components/ReceiptForm'
import { PatternForm, patternPreviewUrl } from './components/PatternForm'
import { WeaveForm, weavePreviewUrl } from './components/WeaveForm'
import { PhotoForm, photoPreviewUrl } from './components/PhotoForm'

export const activeTab = signal<'receipt' | 'patterns' | 'weave' | 'photos'>('receipt')

const previewUrls = {
  receipt: () => receiptPreviewUrl.value,
  patterns: () => patternPreviewUrl.value,
  weave: () => weavePreviewUrl.value,
  photos: () => photoPreviewUrl.value,
}

const placeholderTexts = {
  receipt: 'Start typing to see preview...',
  patterns: 'Select a pattern to see preview...',
  weave: 'Add at least 2 patterns to see preview...',
  photos: 'Upload an image to see preview...',
}

export function App() {
  const previewUrl = previewUrls[activeTab.value]()
  const placeholderText = placeholderTexts[activeTab.value]

  return (
    <div class="container">
      <h1>Estrella Printer</h1>
      <p class="subtitle">Print text receipts or visual patterns to your thermal printer</p>
      <Tabs />
      <div class="main-layout">
        <div class="form-panel">
          {activeTab.value === 'receipt' ? (
            <ReceiptForm />
          ) : activeTab.value === 'patterns' ? (
            <PatternForm />
          ) : activeTab.value === 'weave' ? (
            <WeaveForm />
          ) : (
            <PhotoForm />
          )}
        </div>
        <div class="preview-panel">
          <h3>Preview</h3>
          <div class="preview-container">
            {previewUrl ? (
              <img src={previewUrl} alt="Preview" />
            ) : (
              <div class="loading">{placeholderText}</div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
