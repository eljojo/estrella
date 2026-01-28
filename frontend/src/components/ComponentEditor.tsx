import { signal } from '@preact/signals'
import { useState, useEffect } from 'preact/hooks'
import { fetchPatterns, fetchParams, fetchRandomParams, fetchDefaultComponent, ParamSpec } from '../api'
import { ParamInput } from './PatternForm'

// Pattern list (shared across editors)
const patternsList = signal<string[]>([])
let patternsFetched = false

export function ensurePatternsFetched() {
  if (!patternsFetched) {
    patternsFetched = true
    fetchPatterns()
      .then((p) => {
        if (Array.isArray(p)) patternsList.value = p.sort()
      })
      .catch(console.error)
  }
}

// Component type registry — injected by the backend into index.html
export const COMPONENT_TYPES: ReadonlyArray<{ type: string; label: string }> =
  (window as any).__COMPONENT_TYPES ?? []

export function getComponentLabel(type: string): string {
  return COMPONENT_TYPES.find((t) => t.type === type)?.label || type
}

export async function createDefaultComponent(type: string): Promise<any> {
  return fetchDefaultComponent(type)
}

function truncate(s: string | undefined, max: number): string {
  if (!s) return ''
  return s.length > max ? s.substring(0, max) + '...' : s
}

export function getComponentSummary(comp: any): string {
  if (!comp) return ''
  switch (comp.type) {
    case 'text':
      return truncate(comp.content, 30)
    case 'header':
      return truncate(comp.content, 30)
    case 'banner':
      return truncate(comp.content, 25)
    case 'divider':
      return comp.style || 'dashed'
    case 'spacer':
      return comp.mm != null ? `${comp.mm}mm` : comp.lines != null ? `${comp.lines} lines` : ''
    case 'blank_line':
      return ''
    case 'columns':
      return `${truncate(comp.left, 12)} | ${truncate(comp.right, 12)}`
    case 'line_item':
      return `${comp.name} $${Number(comp.price || 0).toFixed(2)}`
    case 'total':
      return `$${Number(comp.amount || 0).toFixed(2)}`
    case 'table':
      return `${comp.headers?.length || 0} cols, ${comp.rows?.length || 0} rows`
    case 'markdown':
      return truncate(comp.content, 30)
    case 'chart':
      return `${comp.style} chart`
    case 'qr_code':
      return truncate(comp.data, 20)
    case 'pdf417':
      return truncate(comp.data, 20)
    case 'barcode':
      return `${comp.format}: ${truncate(comp.data, 15)}`
    case 'pattern':
      return comp.name || ''
    case 'image':
      return truncate(comp.url, 25) || '(no URL)'
    case 'canvas':
      return `${comp.elements?.length || 0} elements`
    case 'nv_logo':
      return `key: ${comp.key}`
    default:
      return ''
  }
}

// ============================================================================
// Main Component Editor (dispatcher)
// ============================================================================

interface ComponentEditorProps {
  component: any
  onUpdate: (updates: any) => void
  canvasElementIndex?: number | null
  onCanvasElementSelect?: (index: number | null) => void
}

export function ComponentEditor({ component, onUpdate, canvasElementIndex, onCanvasElementSelect }: ComponentEditorProps) {
  switch (component?.type) {
    case 'text':
      return <TextEditor comp={component} onUpdate={onUpdate} />
    case 'header':
      return <HeaderEditor comp={component} onUpdate={onUpdate} />
    case 'banner':
      return <BannerEditor comp={component} onUpdate={onUpdate} />
    case 'divider':
      return <DividerEditor comp={component} onUpdate={onUpdate} />
    case 'spacer':
      return <SpacerEditor comp={component} onUpdate={onUpdate} />
    case 'blank_line':
      return <p class="hint">No configurable options.</p>
    case 'columns':
      return <ColumnsEditor comp={component} onUpdate={onUpdate} />
    case 'line_item':
      return <LineItemEditor comp={component} onUpdate={onUpdate} />
    case 'total':
      return <TotalEditor comp={component} onUpdate={onUpdate} />
    case 'table':
      return <TableEditor comp={component} onUpdate={onUpdate} />
    case 'markdown':
      return <MarkdownEditor comp={component} onUpdate={onUpdate} />
    case 'chart':
      return <ChartEditor comp={component} onUpdate={onUpdate} />
    case 'qr_code':
      return <QrCodeEditor comp={component} onUpdate={onUpdate} />
    case 'pdf417':
      return <Pdf417Editor comp={component} onUpdate={onUpdate} />
    case 'barcode':
      return <BarcodeEditor comp={component} onUpdate={onUpdate} />
    case 'pattern':
      return <PatternEditor comp={component} onUpdate={onUpdate} />
    case 'image':
      return <ImageEditor comp={component} onUpdate={onUpdate} />
    case 'canvas':
      return <CanvasEditor comp={component} onUpdate={onUpdate} expandedElement={canvasElementIndex} onElementSelect={onCanvasElementSelect} />
    case 'nv_logo':
      return <NvLogoEditor comp={component} onUpdate={onUpdate} />
    default:
      return <JsonFallbackEditor comp={component} onUpdate={onUpdate} />
  }
}

// ============================================================================
// Helpers
// ============================================================================

type EditorProps = { comp: any; onUpdate: (u: any) => void }

function BoolToggle({
  label,
  checked,
  onChange,
}: {
  label: string
  checked: boolean
  onChange: (v: boolean) => void
}) {
  return (
    <label class="toggle-label">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange((e.target as HTMLInputElement).checked)}
      />
      {label}
    </label>
  )
}

const BLEND_MODES = ['normal', 'multiply', 'screen', 'overlay', 'add', 'difference', 'min', 'max']

// ============================================================================
// Type-specific editors
// ============================================================================

function TextEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>Content</label>
        <textarea
          value={comp.content || ''}
          onInput={(e) => onUpdate({ content: (e.target as HTMLTextAreaElement).value })}
          rows={3}
          class="component-textarea"
        />
      </div>
      <div class="style-toggles">
        <BoolToggle label="Bold" checked={!!comp.bold} onChange={(v) => onUpdate({ bold: v || undefined })} />
        <BoolToggle
          label="Center"
          checked={!!comp.center}
          onChange={(v) => onUpdate({ center: v || undefined })}
        />
        <BoolToggle label="Right" checked={!!comp.right} onChange={(v) => onUpdate({ right: v || undefined })} />
        <BoolToggle
          label="Underline"
          checked={!!comp.underline}
          onChange={(v) => onUpdate({ underline: v || undefined })}
        />
        <BoolToggle
          label="Invert"
          checked={!!comp.invert}
          onChange={(v) => onUpdate({ invert: v || undefined })}
        />
      </div>
      <div class="editor-row">
        <div class="form-group">
          <label>Size</label>
          <select
            value={comp.size ?? 1}
            onChange={(e) => onUpdate({ size: parseInt((e.target as HTMLSelectElement).value) })}
          >
            <option value={0}>Small (Font B)</option>
            <option value={1}>Normal</option>
            <option value={2}>Double</option>
            <option value={3}>Triple</option>
          </select>
        </div>
        <div class="form-group">
          <label>Font</label>
          <select
            value={comp.font || ''}
            onChange={(e) => onUpdate({ font: (e.target as HTMLSelectElement).value || undefined })}
          >
            <option value="">Default</option>
            <option value="ibm">IBM Plex Sans</option>
          </select>
        </div>
      </div>
    </div>
  )
}

function HeaderEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>Content</label>
        <input
          type="text"
          value={comp.content || ''}
          onInput={(e) => onUpdate({ content: (e.target as HTMLInputElement).value })}
        />
        <p class="hint">Pre-styled: centered, bold, double-width</p>
      </div>
    </div>
  )
}

function BannerEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>Content</label>
        <input
          type="text"
          value={comp.content || ''}
          onInput={(e) => onUpdate({ content: (e.target as HTMLInputElement).value })}
        />
      </div>
      <div class="editor-row">
        <div class="form-group">
          <label>Border</label>
          <select
            value={comp.border || 'single'}
            onChange={(e) => onUpdate({ border: (e.target as HTMLSelectElement).value })}
          >
            <option value="single">Single</option>
            <option value="double">Double</option>
            <option value="heavy">Heavy</option>
            <option value="shade">Shade</option>
            <option value="shadow">Shadow</option>
            <option value="mixed">Mixed</option>
            <option value="rule">Rule</option>
            <option value="heading">Heading</option>
            <option value="tag">Tag</option>
          </select>
        </div>
        <div class="form-group">
          <label>Size</label>
          <select
            value={comp.size ?? 1}
            onChange={(e) => onUpdate({ size: parseInt((e.target as HTMLSelectElement).value) })}
          >
            <option value={0}>Small</option>
            <option value={1}>Normal</option>
            <option value={2}>Double</option>
            <option value={3}>Triple</option>
          </select>
        </div>
        <div class="form-group">
          <label>Font</label>
          <select
            value={comp.font || ''}
            onChange={(e) => onUpdate({ font: (e.target as HTMLSelectElement).value || undefined })}
          >
            <option value="">Default</option>
            <option value="ibm">IBM Plex Sans</option>
          </select>
        </div>
      </div>
    </div>
  )
}

function DividerEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>Style</label>
        <select
          value={comp.style || 'dashed'}
          onChange={(e) => onUpdate({ style: (e.target as HTMLSelectElement).value || undefined })}
        >
          <option value="dashed">Dashed (default)</option>
          <option value="solid">Solid</option>
          <option value="double">Double</option>
          <option value="equals">Equals</option>
        </select>
      </div>
    </div>
  )
}

function SpacerEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="editor-row">
        <div class="form-group">
          <label>Height (mm)</label>
          <input
            type="number"
            step="0.5"
            min="0"
            value={comp.mm ?? ''}
            onInput={(e) => {
              const v = parseFloat((e.target as HTMLInputElement).value)
              if (!isNaN(v)) onUpdate({ mm: v, lines: undefined })
            }}
          />
        </div>
        <div class="form-group">
          <label>Lines</label>
          <input
            type="number"
            min="0"
            value={comp.lines ?? ''}
            onInput={(e) => {
              const v = parseInt((e.target as HTMLInputElement).value)
              if (!isNaN(v)) onUpdate({ lines: v, mm: undefined })
            }}
          />
        </div>
      </div>
      <p class="hint">Set mm or lines (not both)</p>
    </div>
  )
}

function ColumnsEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="editor-row">
        <div class="form-group">
          <label>Left</label>
          <input
            type="text"
            value={comp.left || ''}
            onInput={(e) => onUpdate({ left: (e.target as HTMLInputElement).value })}
          />
        </div>
        <div class="form-group">
          <label>Right</label>
          <input
            type="text"
            value={comp.right || ''}
            onInput={(e) => onUpdate({ right: (e.target as HTMLInputElement).value })}
          />
        </div>
      </div>
      <div class="style-toggles">
        <BoolToggle label="Bold" checked={!!comp.bold} onChange={(v) => onUpdate({ bold: v || undefined })} />
      </div>
    </div>
  )
}

function LineItemEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="editor-row">
        <div class="form-group">
          <label>Name</label>
          <input
            type="text"
            value={comp.name || ''}
            onInput={(e) => onUpdate({ name: (e.target as HTMLInputElement).value })}
          />
        </div>
        <div class="form-group">
          <label>Price</label>
          <input
            type="number"
            step="0.01"
            value={comp.price ?? 0}
            onInput={(e) => onUpdate({ price: parseFloat((e.target as HTMLInputElement).value) || 0 })}
          />
        </div>
      </div>
    </div>
  )
}

function TotalEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>Amount</label>
        <input
          type="number"
          step="0.01"
          value={comp.amount ?? 0}
          onInput={(e) => onUpdate({ amount: parseFloat((e.target as HTMLInputElement).value) || 0 })}
        />
      </div>
    </div>
  )
}

function TableEditor({ comp, onUpdate }: EditorProps) {
  const headers: string[] = comp.headers || []
  const rows: string[][] = comp.rows || []

  return (
    <div class="component-editor">
      <div class="editor-row">
        <div class="form-group">
          <label>Border</label>
          <select
            value={comp.border || 'single'}
            onChange={(e) => onUpdate({ border: (e.target as HTMLSelectElement).value })}
          >
            <option value="single">Single</option>
            <option value="double">Double</option>
            <option value="heavy">Heavy</option>
            <option value="mixed">Mixed</option>
            <option value="shade">Shade</option>
          </select>
        </div>
      </div>
      <div class="style-toggles">
        <BoolToggle
          label="Row Separator"
          checked={!!comp.row_separator}
          onChange={(v) => onUpdate({ row_separator: v || undefined })}
        />
      </div>
      <div class="form-group">
        <label>Headers (comma-separated)</label>
        <input
          type="text"
          value={headers.join(', ')}
          onInput={(e) =>
            onUpdate({ headers: (e.target as HTMLInputElement).value.split(',').map((s) => s.trim()) })
          }
        />
      </div>
      <div class="form-group">
        <label>Rows</label>
        {rows.map((row, i) => (
          <div key={i} class="table-row-editor">
            <input
              type="text"
              value={row.join(', ')}
              onInput={(e) => {
                const newRow = (e.target as HTMLInputElement).value.split(',').map((s) => s.trim())
                const newRows = [...rows]
                newRows[i] = newRow
                onUpdate({ rows: newRows })
              }}
            />
            <button
              type="button"
              class="icon-btn delete"
              onClick={() => onUpdate({ rows: rows.filter((_: any, j: number) => j !== i) })}
            >
              &times;
            </button>
          </div>
        ))}
        <button
          type="button"
          class="add-layer-btn"
          onClick={() => onUpdate({ rows: [...rows, headers.map(() => '')] })}
        >
          + Add Row
        </button>
      </div>
    </div>
  )
}

function MarkdownEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>Content</label>
        <textarea
          value={comp.content || ''}
          onInput={(e) => onUpdate({ content: (e.target as HTMLTextAreaElement).value })}
          rows={6}
          class="component-textarea"
        />
        <p class="hint">Supports headings, bold, italic, lists</p>
      </div>
    </div>
  )
}

function ChartEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="editor-row">
        <div class="form-group">
          <label>Style</label>
          <select
            value={comp.style || 'bar'}
            onChange={(e) => onUpdate({ style: (e.target as HTMLSelectElement).value })}
          >
            <option value="line">Line</option>
            <option value="area">Area</option>
            <option value="bar">Bar</option>
            <option value="dot">Dot</option>
          </select>
        </div>
        <div class="form-group">
          <label>Height (px)</label>
          <input
            type="number"
            min="50"
            value={comp.height || 100}
            onInput={(e) => onUpdate({ height: parseInt((e.target as HTMLInputElement).value) || 100 })}
          />
        </div>
        <div class="form-group">
          <label>Y Suffix</label>
          <input
            type="text"
            value={comp.y_suffix || ''}
            onInput={(e) => onUpdate({ y_suffix: (e.target as HTMLInputElement).value || undefined })}
          />
        </div>
      </div>
      <div class="form-group">
        <label>Labels (comma-separated)</label>
        <input
          type="text"
          value={(comp.labels || []).join(', ')}
          onInput={(e) =>
            onUpdate({ labels: (e.target as HTMLInputElement).value.split(',').map((s: string) => s.trim()) })
          }
        />
      </div>
      <div class="form-group">
        <label>Values (comma-separated)</label>
        <input
          type="text"
          value={(comp.values || []).join(', ')}
          onInput={(e) =>
            onUpdate({
              values: (e.target as HTMLInputElement).value
                .split(',')
                .map((s: string) => parseFloat(s.trim()) || 0),
            })
          }
        />
      </div>
    </div>
  )
}

function QrCodeEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>Data</label>
        <input
          type="text"
          value={comp.data || ''}
          onInput={(e) => onUpdate({ data: (e.target as HTMLInputElement).value })}
        />
      </div>
    </div>
  )
}

function Pdf417Editor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>Data</label>
        <input
          type="text"
          value={comp.data || ''}
          onInput={(e) => onUpdate({ data: (e.target as HTMLInputElement).value })}
        />
      </div>
    </div>
  )
}

function BarcodeEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="editor-row">
        <div class="form-group">
          <label>Format</label>
          <select
            value={comp.format || 'code128'}
            onChange={(e) => onUpdate({ format: (e.target as HTMLSelectElement).value })}
          >
            <option value="code39">Code 39</option>
            <option value="code128">Code 128</option>
            <option value="ean13">EAN-13</option>
            <option value="upca">UPC-A</option>
            <option value="itf">ITF</option>
          </select>
        </div>
        <div class="form-group">
          <label>Height (px)</label>
          <input
            type="number"
            min="10"
            value={comp.height || 60}
            onInput={(e) => onUpdate({ height: parseInt((e.target as HTMLInputElement).value) || 60 })}
          />
        </div>
      </div>
      <div class="form-group">
        <label>Data</label>
        <input
          type="text"
          value={comp.data || ''}
          onInput={(e) => onUpdate({ data: (e.target as HTMLInputElement).value })}
        />
      </div>
    </div>
  )
}

function PatternEditor({ comp, onUpdate }: EditorProps) {
  ensurePatternsFetched()
  const [specs, setSpecs] = useState<ParamSpec[]>([])

  // Fetch param specs when pattern name changes
  useEffect(() => {
    if (!comp.name) return
    fetchParams(comp.name)
      .then((info) => {
        setSpecs(info.specs)
        // Initialize defaults if component has no params yet
        if (!comp.params || Object.keys(comp.params).length === 0) {
          onUpdate({ params: info.params })
        }
      })
      .catch(console.error)
  }, [comp.name])

  const handleParamChange = (name: string, value: string) => {
    onUpdate({ params: { ...(comp.params || {}), [name]: value } })
  }

  const handleRandomize = async () => {
    if (!comp.name) return
    try {
      const info = await fetchRandomParams(comp.name)
      onUpdate({ params: info.params })
      setSpecs(info.specs)
    } catch (err) {
      console.error('Failed to randomize:', err)
    }
  }

  return (
    <div class="component-editor">
      <div class="editor-row">
        <div class="form-group">
          <label>Pattern</label>
          <select
            value={comp.name || ''}
            onChange={(e) => onUpdate({ name: (e.target as HTMLSelectElement).value, params: {} })}
          >
            {patternsList.value.length === 0 ? (
              <option value={comp.name}>{comp.name}</option>
            ) : (
              patternsList.value.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))
            )}
          </select>
        </div>
        <div class="form-group">
          <label>Height (px)</label>
          <input
            type="number"
            min="1"
            value={comp.height || 80}
            onInput={(e) => onUpdate({ height: parseInt((e.target as HTMLInputElement).value) || 80 })}
          />
        </div>
        <div class="form-group">
          <label>Dither</label>
          <select
            value={comp.dither || ''}
            onChange={(e) => onUpdate({ dither: (e.target as HTMLSelectElement).value || undefined })}
          >
            <option value="">Default</option>
            <option value="jarvis">Jarvis</option>
            <option value="atkinson">Atkinson</option>
            <option value="bayer">Bayer</option>
            <option value="floyd-steinberg">Floyd-Steinberg</option>
          </select>
        </div>
      </div>

      {specs.length > 0 && (
        <div class="form-group">
          <label>Parameters</label>
          <div class="params-grid">
            {specs.map((spec) => (
              <ParamInput
                key={spec.name}
                spec={spec}
                value={(comp.params || {})[spec.name] || ''}
                onChange={(v) => handleParamChange(spec.name, v)}
              />
            ))}
          </div>
        </div>
      )}

      <div class="button-row">
        <button
          type="button"
          class="button-secondary"
          onClick={handleRandomize}
          disabled={!comp.name}
        >
          Randomize
        </button>
      </div>
    </div>
  )
}

function ImageEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>URL</label>
        <input
          type="text"
          placeholder="https://example.com/image.png"
          value={comp.url || ''}
          onInput={(e) => onUpdate({ url: (e.target as HTMLInputElement).value })}
        />
      </div>
      <div class="editor-row">
        <div class="form-group">
          <label>Width</label>
          <input
            type="number"
            min="0"
            value={comp.width ?? ''}
            onInput={(e) => {
              const v = parseInt((e.target as HTMLInputElement).value)
              onUpdate({ width: isNaN(v) ? undefined : v })
            }}
          />
        </div>
        <div class="form-group">
          <label>Height</label>
          <input
            type="number"
            min="0"
            value={comp.height ?? ''}
            onInput={(e) => {
              const v = parseInt((e.target as HTMLInputElement).value)
              onUpdate({ height: isNaN(v) ? undefined : v })
            }}
          />
        </div>
        <div class="form-group">
          <label>Align</label>
          <select
            value={comp.align || 'center'}
            onChange={(e) => onUpdate({ align: (e.target as HTMLSelectElement).value })}
          >
            <option value="left">Left</option>
            <option value="center">Center</option>
            <option value="right">Right</option>
          </select>
        </div>
      </div>
    </div>
  )
}

function NvLogoEditor({ comp, onUpdate }: EditorProps) {
  return (
    <div class="component-editor">
      <div class="form-group">
        <label>Key</label>
        <input
          type="text"
          value={comp.key || ''}
          onInput={(e) => onUpdate({ key: (e.target as HTMLInputElement).value })}
        />
      </div>
      <div class="style-toggles">
        <BoolToggle
          label="Center"
          checked={!!comp.center}
          onChange={(v) => onUpdate({ center: v || undefined })}
        />
      </div>
    </div>
  )
}

// ============================================================================
// Canvas editor (nested elements)
// ============================================================================

function CanvasEditor({ comp, onUpdate, expandedElement, onElementSelect }: EditorProps & {
  expandedElement?: number | null
  onElementSelect?: (index: number | null) => void
}) {
  const elements: any[] = comp.elements || []
  // Use external expansion control when provided, otherwise local state
  const [localExpanded, setLocalExpanded] = useState<number | null>(null)
  const expandedIdx = onElementSelect ? expandedElement : localExpanded
  const setExpandedIdx = onElementSelect || setLocalExpanded

  const updateElement = (index: number, updates: any) => {
    const newElements = [...elements]
    newElements[index] = { ...newElements[index], ...updates }
    onUpdate({ elements: newElements })
  }

  const removeElement = (index: number) => {
    onUpdate({ elements: elements.filter((_: any, i: number) => i !== index) })
    if (expandedIdx === index) setExpandedIdx(null)
  }

  const addElement = async (type: string) => {
    const comp = await createDefaultComponent(type)
    onUpdate({ elements: [...elements, comp] })
  }

  const moveElement = (index: number, direction: 'up' | 'down') => {
    const newIndex = direction === 'up' ? index - 1 : index + 1
    if (newIndex < 0 || newIndex >= elements.length) return
    const newElements = [...elements]
    ;[newElements[index], newElements[newIndex]] = [newElements[newIndex], newElements[index]]
    onUpdate({ elements: newElements })
    if (expandedIdx === index) setExpandedIdx(newIndex)
  }

  return (
    <div class="component-editor">
      <div class="editor-row">
        <div class="form-group">
          <label>Height (px)</label>
          <input
            type="number"
            min="0"
            value={comp.height ?? ''}
            onInput={(e) => {
              const v = parseInt((e.target as HTMLInputElement).value)
              onUpdate({ height: isNaN(v) ? undefined : v })
            }}
          />
          <p class="hint">Leave empty for auto-height</p>
        </div>
        <div class="form-group">
          <label>Dither</label>
          <select
            value={comp.dither || ''}
            onChange={(e) => onUpdate({ dither: (e.target as HTMLSelectElement).value || undefined })}
          >
            <option value="">Auto</option>
            <option value="none">None</option>
            <option value="atkinson">Atkinson</option>
            <option value="bayer">Bayer</option>
            <option value="floyd-steinberg">Floyd-Steinberg</option>
          </select>
        </div>
      </div>
      <div class="form-group">
        <label>Elements ({elements.length})</label>
        <div class="layers-list">
          {elements.map((el: any, i: number) => (
            <CanvasElementItem
              key={i}
              element={el}
              index={i}
              total={elements.length}
              isExpanded={expandedIdx === i}
              onToggleExpand={() => setExpandedIdx(expandedIdx === i ? null : i)}
              onUpdate={(updates: any) => updateElement(i, updates)}
              onRemove={() => removeElement(i)}
              onMove={(dir: 'up' | 'down') => moveElement(i, dir)}
            />
          ))}
          <select
            class="weave-add-select"
            value=""
            onChange={(e) => {
              const type = (e.target as HTMLSelectElement).value
              if (type) {
                addElement(type)
                ;(e.target as HTMLSelectElement).value = ''
              }
            }}
          >
            <option value="">+ Add Element</option>
            {COMPONENT_TYPES.filter((t) => t.type !== 'canvas').map((t) => (
              <option key={t.type} value={t.type}>
                {t.label}
              </option>
            ))}
          </select>
        </div>
      </div>
    </div>
  )
}

function CanvasElementItem({
  element,
  index,
  total,
  isExpanded,
  onToggleExpand,
  onUpdate,
  onRemove,
  onMove,
}: {
  element: any
  index: number
  total: number
  isExpanded: boolean
  onToggleExpand: () => void
  onUpdate: (updates: any) => void
  onRemove: () => void
  onMove: (dir: 'up' | 'down') => void
}) {
  const hasPosition = element.position != null

  return (
    <div class={`layer-item ${isExpanded ? 'selected' : ''}`} style={{ flexDirection: 'column', alignItems: 'stretch' }}>
      <div
        style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}
        onClick={onToggleExpand}
      >
        <span class="layer-name">
          {index + 1}. {getComponentLabel(element.type)}
        </span>
        <span class="layer-meta">
          <span class="layer-blend">{getComponentSummary(element)}</span>
          {hasPosition && (
            <span class="layer-opacity">
              ({element.position.x}, {element.position.y})
            </span>
          )}
        </span>
        <div class="layer-actions">
          <button
            type="button"
            class="icon-btn"
            onClick={(e) => {
              e.stopPropagation()
              onMove('up')
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
              onMove('down')
            }}
            disabled={index === total - 1}
            title="Move down"
          >
            &darr;
          </button>
          <button
            type="button"
            class="icon-btn delete"
            onClick={(e) => {
              e.stopPropagation()
              onRemove()
            }}
            title="Remove"
          >
            &times;
          </button>
        </div>
      </div>
      {isExpanded && (
        <div class="layer-editor" style={{ marginTop: '8px' }}>
          {hasPosition ? (
            <div class="editor-row">
              <div class="form-group">
                <label>X</label>
                <input
                  type="number"
                  value={element.position?.x ?? 0}
                  onInput={(e) =>
                    onUpdate({
                      position: { ...element.position, x: parseInt((e.target as HTMLInputElement).value) || 0 },
                    })
                  }
                />
              </div>
              <div class="form-group">
                <label>Y</label>
                <input
                  type="number"
                  value={element.position?.y ?? 0}
                  onInput={(e) =>
                    onUpdate({
                      position: { ...element.position, y: parseInt((e.target as HTMLInputElement).value) || 0 },
                    })
                  }
                />
              </div>
              <div class="form-group">
                <label>&nbsp;</label>
                <button
                  type="button"
                  class="button-secondary"
                  style={{ padding: '8px 12px', fontSize: '13px' }}
                  onClick={() => onUpdate({ position: undefined })}
                >
                  Remove Position
                </button>
              </div>
            </div>
          ) : (
            <button
              type="button"
              class="add-layer-btn"
              style={{ marginBottom: '8px' }}
              onClick={() => onUpdate({ position: { x: 0, y: 0 } })}
            >
              Add Position (switch to absolute)
            </button>
          )}
          <div class="editor-row">
            <div class="form-group">
              <label>Blend Mode</label>
              <select
                value={element.blend_mode || 'normal'}
                onChange={(e) => onUpdate({ blend_mode: (e.target as HTMLSelectElement).value })}
              >
                {BLEND_MODES.map((m) => (
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
                value={element.opacity ?? 1}
                onInput={(e) => onUpdate({ opacity: parseFloat((e.target as HTMLInputElement).value) })}
              />
              <span class="opacity-value">{((element.opacity ?? 1) * 100).toFixed(0)}%</span>
            </div>
          </div>
          <ComponentEditor component={element} onUpdate={onUpdate} />
        </div>
      )}
    </div>
  )
}

// ============================================================================
// JSON fallback (for unknown component types)
// ============================================================================

function JsonFallbackEditor({ comp, onUpdate }: EditorProps) {
  const [text, setText] = useState(JSON.stringify(comp, null, 2))
  const [error, setError] = useState<string | null>(null)

  return (
    <div class="component-editor">
      <div class="form-group">
        <label>JSON</label>
        <textarea
          value={text}
          onInput={(e) => {
            const val = (e.target as HTMLTextAreaElement).value
            setText(val)
            try {
              const parsed = JSON.parse(val)
              setError(null)
              // Pass full object — merge will overwrite all keys
              onUpdate(parsed)
            } catch (err) {
              setError((err as Error).message)
            }
          }}
          rows={8}
          class="component-textarea"
        />
        {error && <p class="hint error-hint">{error}</p>}
      </div>
    </div>
  )
}
