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
import {
  EditorForm,
  editorPreviewUrl,
  editorCustomized,
  cut as editorCut,
  editorCanPrint,
  loading as editorLoading,
  triggerEditorPrint,
  editorCanvasOverlay,
  editorCanvasElementIndex,
  handleCanvasOverlaySelect,
  handleCanvasOverlayUpdate,
  handleCanvasOverlayDoubleClick,
} from './components/EditorForm'
import { PhotoForm, photoPreviewUrl, photoGrayscaleActive, handlePhotoDrop } from './components/PhotoForm'
import { JsonForm, jsonPreviewUrl, jsonCustomized } from './components/JsonForm'
import { PrintOptions } from './components/PrintOptions'
import { LayerCanvas } from './components/LayerCanvas'

export const activeTab = signal<'receipt' | 'patterns' | 'weave' | 'composer' | 'photos' | 'json'>('photos')

const previewUrls = {
  receipt: () => receiptPreviewUrl.value,
  patterns: () => patternPreviewUrl.value,
  weave: () => weavePreviewUrl.value,
  composer: () => editorPreviewUrl.value,
  photos: () => photoPreviewUrl.value,
  json: () => jsonPreviewUrl.value,
}

const placeholderTexts = {
  receipt: 'Start typing to see preview...',
  patterns: 'Select a pattern to see preview...',
  weave: 'Add at least 2 patterns to see preview...',
  composer: 'Add components to see preview...',
  photos: 'Upload an image to see preview...',
  json: 'Edit JSON to see preview...',
}

const grayscaleStates = {
  receipt: () => receiptCustomized.value,
  patterns: () => patternCustomized.value,
  weave: () => weaveHasBlend.value,
  composer: () => editorCustomized.value,
  photos: () => photoGrayscaleActive.value,
  json: () => jsonCustomized.value,
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
      // Only switch to photos for external file drags (not internal element drags)
      // Check that this is an external drag by verifying items exist and are files
      const items = e.dataTransfer?.items
      if (items && items.length > 0) {
        const hasFiles = Array.from(items).some(item => item.kind === 'file')
        if (hasFiles) {
          activeTab.value = 'photos'
        }
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
    <div class={`container${activeTab.value === 'json' ? ' container--wide' : ''}`}>
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
          ) : activeTab.value === 'composer' ? (
            <EditorForm />
          ) : activeTab.value === 'json' ? (
            <JsonForm />
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
              {activeTab.value === 'composer' && editorCanvasOverlay.value && editorCanvasOverlay.value.documentHeight > 0 && (
                <div
                  class="canvas-overlay-wrapper"
                  style={{
                    position: 'absolute',
                    left: '5%',
                    width: '90%',
                    top: `${(editorCanvasOverlay.value.yOffset / editorCanvasOverlay.value.documentHeight) * 100}%`,
                    height: `${(editorCanvasOverlay.value.canvasHeight / editorCanvasOverlay.value.documentHeight) * 100}%`,
                  }}
                >
                  <LayerCanvas
                    layers={editorCanvasOverlay.value.layers}
                    selectedIndex={editorCanvasElementIndex.value}
                    canvasWidth={editorCanvasOverlay.value.canvasWidth}
                    canvasHeight={editorCanvasOverlay.value.canvasHeight}
                    onSelect={handleCanvasOverlaySelect}
                    onUpdate={handleCanvasOverlayUpdate}
                    onDoubleClick={handleCanvasOverlayDoubleClick}
                  />
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
          {activeTab.value === 'composer' && (
            <>
              <PrintOptions cut={editorCut} />
              <button
                type="button"
                class="print-button"
                onClick={() => triggerEditorPrint()}
                disabled={!editorCanPrint.value || editorLoading.value}
              >
                {editorLoading.value ? 'Printing...' : 'Print'}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
