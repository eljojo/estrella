import { Signal } from '@preact/signals'

export function PrintOptions({
  cut,
  printDetails,
  detailsLabel,
}: {
  cut: Signal<boolean>
  printDetails?: Signal<boolean>
  detailsLabel?: string
}) {
  return (
    <div class="form-group checkbox-group receipt-options">
      <label>
        <input
          type="checkbox"
          checked={cut.value}
          onChange={(e) => (cut.value = (e.target as HTMLInputElement).checked)}
        />
        Cut page after printing
      </label>
      {printDetails && detailsLabel && (
        <label>
          <input
            type="checkbox"
            checked={printDetails.value}
            onChange={(e) => (printDetails.value = (e.target as HTMLInputElement).checked)}
          />
          {detailsLabel}
        </label>
      )}
    </div>
  )
}
