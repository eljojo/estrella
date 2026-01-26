import { activeTab } from '../App'

export function Tabs() {
  return (
    <div class="tabs">
      <button
        class={`tab ${activeTab.value === 'photos' ? 'active' : ''}`}
        onClick={() => (activeTab.value = 'photos')}
      >
        Photos
      </button>
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
      <button
        class={`tab ${activeTab.value === 'weave' ? 'active' : ''}`}
        onClick={() => (activeTab.value = 'weave')}
      >
        Weave
      </button>
      <button
        class={`tab ${activeTab.value === 'composer' ? 'active' : ''}`}
        onClick={() => (activeTab.value = 'composer')}
      >
        Composer
      </button>
      <button
        class={`tab ${activeTab.value === 'json' ? 'active' : ''}`}
        onClick={() => (activeTab.value = 'json')}
      >
        JSON API
      </button>
    </div>
  )
}
