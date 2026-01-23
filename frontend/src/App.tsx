import { signal, effect } from '@preact/signals'
import { useEffect } from 'preact/hooks'
import { Tabs } from './components/Tabs'
import {
  ReceiptForm,
  receiptPreviewUrl,
  cut as receiptCut,
  printDetails as receiptPrintDetails,
  receiptCustomized,
} from './components/ReceiptForm'
import {
  PatternForm,
  patternPreviewUrl,
  cut as patternCut,
  printDetails as patternPrintDetails,
  patternCustomized,
} from './components/PatternForm'
import {
  WeaveForm,
  weavePreviewUrl,
  cut as weaveCut,
  printDetails as weavePrintDetails,
  weaveHasBlend,
} from './components/WeaveForm'
import { PhotoForm, photoPreviewUrl, photoGrayscaleActive, handlePhotoDrop } from './components/PhotoForm'
import { PrintOptions } from './components/PrintOptions'

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

const grayscaleStates = {
  receipt: () => receiptCustomized.value,
  patterns: () => patternCustomized.value,
  weave: () => weaveHasBlend.value,
  photos: () => photoGrayscaleActive.value,
}

effect(() => {
  const active = grayscaleStates[activeTab.value]()
  document.documentElement.classList.toggle('grayscale', active)
})

export function App() {
  const previewUrl = previewUrls[activeTab.value]()
  const placeholderText = placeholderTexts[activeTab.value]

  useEffect(() => {
    const root = document.getElementById('app')
    if (!root) return

    const handleDragEnter = (e: DragEvent) => {
      const types = e.dataTransfer?.types
      if (types && Array.from(types).includes('Files')) {
        activeTab.value = 'photos'
      }
    }

    const handleDragOver = (e: DragEvent) => {
      if (activeTab.value !== 'photos') return
      e.preventDefault()
    }

    const handleDrop = (e: DragEvent) => {
      const file = e.dataTransfer?.files[0]
      if (file && file.type.startsWith('image/')) {
        activeTab.value = 'photos'
        e.preventDefault()
        handlePhotoDrop(file)
      }
    }

    root.addEventListener('dragenter', handleDragEnter)
    root.addEventListener('dragover', handleDragOver)
    root.addEventListener('drop', handleDrop)

    return () => {
      root.removeEventListener('dragenter', handleDragEnter)
      root.removeEventListener('dragover', handleDragOver)
      root.removeEventListener('drop', handleDrop)
    }
  }, [])

  return (
    <div class="container">
      <h1>Estrella ⭐️</h1>
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
            <div class="preview-stage">
              {previewUrl ? (
                <img src={previewUrl} alt="Preview" class="preview-image" />
              ) : (
                <div class="preview-placeholder">
                  <div class="preview-placeholder-text">{placeholderText}</div>
                </div>
              )}
            </div>
          </div>
          {activeTab.value === 'receipt' && (
            <PrintOptions
              cut={receiptCut}
              printDetails={receiptPrintDetails}
              detailsLabel="Print details (date footer)"
            />
          )}
          {activeTab.value === 'patterns' && (
            <PrintOptions
              cut={patternCut}
              printDetails={patternPrintDetails}
              detailsLabel="Print details (title and parameters)"
            />
          )}
          {activeTab.value === 'weave' && (
            <PrintOptions
              cut={weaveCut}
              printDetails={weavePrintDetails}
              detailsLabel="Print details (title and parameters)"
            />
          )}
        </div>
      </div>
    </div>
  )
}
