import { activeTab } from '../App'

export function Tabs() {
  return (
    <div class="tabs">
      <button
        class={`tab ${activeTab.value === 'receipt' ? 'active' : ''}`}
        onClick={() => (activeTab.value = 'receipt')}
      >
        Receipt
      </button>
      <button
        class={`tab ${activeTab.value === 'patterns' ? 'active' : ''}`}
        onClick={() => (activeTab.value = 'patterns')}
      >
        Patterns
      </button>
    </div>
  )
}
