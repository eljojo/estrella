import { signal, computed, effect } from '@preact/signals'
import { fetchProfiles, fetchActiveProfile, setActiveProfile, DeviceProfile } from '../api'

// ===== Shared profile signals =====

const profiles = signal<DeviceProfile[]>([])
export const activeProfile = signal<DeviceProfile | null>(null)

/// Whether the active profile can send to a physical printer.
export const canPrint = computed(() => activeProfile.value?.type === 'printer')

/// Width in dots/pixels of the active profile.
export const profileWidth = computed(() => activeProfile.value?.width ?? 576)

// Fetch profiles on load
effect(() => {
  Promise.all([
    fetchProfiles().then((p) => (profiles.value = p)),
    fetchActiveProfile().then((p) => (activeProfile.value = p)),
  ]).catch((e) => console.error('Failed to fetch profiles:', e))
})

// ===== Component =====

export function ProfileSelector() {
  const handleChange = async (name: string) => {
    try {
      const profile = await setActiveProfile({ name })
      activeProfile.value = profile
    } catch (err) {
      console.error('Failed to set profile:', err)
    }
  }

  if (!activeProfile.value) return null

  return (
    <div class="profile-selector">
      <select
        value={activeProfile.value.name}
        onChange={(e) => handleChange((e.target as HTMLSelectElement).value)}
      >
        {profiles.value.map((p) => (
          <option key={p.name} value={p.name}>
            {p.name} ({p.width}px)
          </option>
        ))}
      </select>
    </div>
  )
}
