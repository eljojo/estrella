/// API client functions for the Estrella backend.

export interface ParamSpec {
  name: string
  label: string
  param_type:
    | { slider: { min: number; max: number; step: number } }
    | { float: { min: number | null; max: number | null; step: number | null } }
    | { int: { min: number | null; max: number | null } }
    | { select: { options: string[] } }
    | 'bool'
  description?: string
}

export interface PatternInfo {
  name: string
  params: Record<string, string>
  specs: ParamSpec[]
}

export interface PrintResult {
  success: boolean
  message?: string
  error?: string
}

/// Fetch the list of available patterns.
export async function fetchPatterns(): Promise<string[]> {
  const response = await fetch('/api/patterns')
  if (!response.ok) throw new Error('Failed to fetch patterns')
  return response.json()
}

/// Fetch golden (default) params for a pattern.
export async function fetchParams(name: string): Promise<PatternInfo> {
  const response = await fetch(`/api/patterns/${name}/params`)
  if (!response.ok) throw new Error('Failed to fetch params')
  return response.json()
}

/// Fetch randomized params for a pattern.
export async function fetchRandomParams(name: string): Promise<PatternInfo> {
  const response = await fetch(`/api/patterns/${name}/randomize`, { method: 'POST' })
  if (!response.ok) throw new Error('Failed to randomize params')
  return response.json()
}

/// Build preview URL for a pattern.
export function buildPreviewUrl(
  name: string,
  lengthMm: number,
  params: Record<string, string>,
  dither: string,
  mode: string
): string {
  const searchParams = new URLSearchParams({
    length_mm: lengthMm.toString(),
    dither,
    mode,
    ...params,
  })
  return `/api/patterns/${name}/preview?${searchParams.toString()}`
}

/// Print a pattern.
export async function printPattern(
  name: string,
  lengthMm: number,
  params: Record<string, string>,
  dither: string,
  mode: string,
  cut: boolean = true,
  printDetails: boolean = true
): Promise<PrintResult> {
  const response = await fetch(`/api/patterns/${name}/print`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      length_mm: lengthMm,
      dither,
      mode,
      params,
      cut,
      print_details: printDetails,
    }),
  })
  return response.json()
}

/// Print a receipt.
export async function printReceipt(
  title: string,
  body: string,
  cut: boolean = true,
  printDetails: boolean = true
): Promise<PrintResult> {
  const response = await fetch('/api/receipt/print', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title: title || null, body, cut, print_details: printDetails }),
  })
  return response.json()
}

/// Fetch receipt preview as a blob URL.
export async function fetchReceiptPreview(
  title: string,
  body: string,
  cut: boolean = true,
  printDetails: boolean = true
): Promise<string> {
  const response = await fetch('/api/receipt/preview', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title: title || null, body, cut, print_details: printDetails }),
  })

  if (!response.ok) {
    throw new Error('Failed to fetch preview')
  }

  const blob = await response.blob()
  return URL.createObjectURL(blob)
}

// ===== JSON API =====

/// Fetch JSON API preview as a blob URL.
export async function fetchJsonPreview(jsonBody: string): Promise<string> {
  const response = await fetch('/api/json/preview', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: jsonBody,
  })

  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || 'Failed to fetch JSON preview')
  }

  const blob = await response.blob()
  return URL.createObjectURL(blob)
}

/// Canvas layout response from the backend.
export interface CanvasLayoutResponse {
  width: number
  height: number
  y_offset: number
  document_height: number
  elements: Array<{ x: number; y: number; width: number; height: number }>
}

/// Fetch canvas layout metadata (element bounding boxes + document positioning).
export async function fetchCanvasLayout(
  document: any[],
  canvasIndex: number,
  cut: boolean = false
): Promise<CanvasLayoutResponse> {
  const response = await fetch('/api/json/canvas-layout', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ document, canvas_index: canvasIndex, cut }),
  })

  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || 'Failed to fetch canvas layout')
  }

  return response.json()
}

/// Print a JSON document.
export async function printJson(jsonBody: string): Promise<PrintResult> {
  const response = await fetch('/api/json/print', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: jsonBody,
  })
  return response.json()
}

// ===== Weave API =====

/// A pattern entry for weave requests.
export interface WeavePatternEntry {
  name: string
  params: Record<string, string>
}

/// Fetch weave preview as a blob URL.
export async function fetchWeavePreview(
  patterns: WeavePatternEntry[],
  lengthMm: number,
  crossfadeMm: number,
  curve: string,
  dither: string,
  mode: string
): Promise<string> {
  const response = await fetch('/api/weave/preview', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      length_mm: lengthMm,
      crossfade_mm: crossfadeMm,
      curve,
      dither,
      mode,
      patterns,
    }),
  })

  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || 'Failed to fetch weave preview')
  }

  const blob = await response.blob()
  return URL.createObjectURL(blob)
}

/// Print a weave.
export async function printWeave(
  patterns: WeavePatternEntry[],
  lengthMm: number,
  crossfadeMm: number,
  curve: string,
  dither: string,
  mode: string,
  cut: boolean = true,
  printDetails: boolean = true
): Promise<PrintResult> {
  const response = await fetch('/api/weave/print', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      length_mm: lengthMm,
      crossfade_mm: crossfadeMm,
      curve,
      dither,
      mode,
      patterns,
      cut,
      print_details: printDetails,
    }),
  })
  return response.json()
}

// ===== Photo API =====

/// Response from photo upload.
export interface PhotoUploadResponse {
  id: string
  filename: string
  width: number
  height: number
  /** True if the image is already binary (1-bit black/white) */
  is_binary: boolean
}

/// Upload an image and get a session ID.
export async function uploadPhoto(imageData: ArrayBuffer, filename: string): Promise<PhotoUploadResponse> {
  const formData = new FormData()
  formData.append('image', new Blob([imageData]), filename)

  const response = await fetch('/api/photo/upload', {
    method: 'POST',
    body: formData,
  })

  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || 'Failed to upload photo')
  }

  return response.json()
}

/// Build preview URL for a photo session.
export function buildPhotoPreviewUrl(
  sessionId: string,
  rotation: number,
  dither: string,
  brightness: number,
  contrast: number,
  cacheKey?: number
): string {
  const searchParams = new URLSearchParams({
    rotation: rotation.toString(),
    dither,
    brightness: brightness.toString(),
    contrast: contrast.toString(),
  })
  // Add cache key if provided to bust browser cache
  if (cacheKey !== undefined) {
    searchParams.set('_t', cacheKey.toString())
  }
  return `/api/photo/${sessionId}/preview?${searchParams.toString()}`
}

/// Print a photo.
export async function printPhoto(
  sessionId: string,
  rotation: number,
  dither: string,
  brightness: number,
  contrast: number,
  mode: string,
  cut: boolean
): Promise<PrintResult> {
  const response = await fetch(`/api/photo/${sessionId}/print`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      rotation,
      dither,
      brightness,
      contrast,
      mode,
      cut,
    }),
  })
  return response.json()
}

// ===== Composer API =====

/// Blend modes available for composer layers.
export type BlendMode = 'normal' | 'multiply' | 'screen' | 'overlay' | 'add' | 'difference' | 'min' | 'max'

/// A layer in a composition.
export interface ComposerLayer {
  pattern: string
  params: Record<string, string>
  x: number
  y: number
  width: number
  height: number
  blend_mode: BlendMode
  opacity: number
}

/// Full composition specification.
export interface ComposerSpec {
  width: number
  height: number
  background: number
  layers: ComposerLayer[]
}

export async function fetchBlendModes(): Promise<string[]> {
  const response = await fetch('/api/composer/blend-modes')
  if (!response.ok) throw new Error('Failed to fetch blend modes')
  const modes: { name: string }[] = await response.json()
  return modes.map((m) => m.name)
}

/// Fetch composer preview as a blob URL.
export async function fetchComposerPreview(spec: ComposerSpec, dither: string): Promise<string> {
  const response = await fetch('/api/composer/preview', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ spec, dither }),
  })

  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || 'Failed to fetch composer preview')
  }

  const blob = await response.blob()
  return URL.createObjectURL(blob)
}

/// Print a composition.
export async function printComposer(
  spec: ComposerSpec,
  dither: string,
  mode: string,
  cut: boolean
): Promise<PrintResult> {
  const response = await fetch('/api/composer/print', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ spec, dither, mode, cut }),
  })
  return response.json()
}
