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
  mode: string
): Promise<PrintResult> {
  const response = await fetch(`/api/patterns/${name}/print`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      length_mm: lengthMm,
      dither,
      mode,
      params,
    }),
  })
  return response.json()
}

/// Print a receipt.
export async function printReceipt(title: string, body: string): Promise<PrintResult> {
  const response = await fetch('/api/receipt/print', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title: title || null, body }),
  })
  return response.json()
}

/// Fetch receipt preview as a blob URL.
export async function fetchReceiptPreview(title: string, body: string): Promise<string> {
  const response = await fetch('/api/receipt/preview', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title: title || null, body }),
  })

  if (!response.ok) {
    throw new Error('Failed to fetch preview')
  }

  const blob = await response.blob()
  return URL.createObjectURL(blob)
}
