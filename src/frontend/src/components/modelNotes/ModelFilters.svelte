<script lang="ts">
  import Input from '../ui/Input.svelte'

  interface Props {
    selectedPlatform?: 'llama' | 'ollama' | 'all'
    showFavoritesOnly?: boolean
    searchQuery?: string
  }

  let {
    selectedPlatform = $bindable('all'),
    showFavoritesOnly = $bindable(false),
    searchQuery = $bindable('')
  }: Props = $props()
</script>

<div class="filters">
  <div class="filter-group">
    <label>Platform:</label>
    <select bind:value={selectedPlatform}>
      <option value="all">All Platforms</option>
      <option value="llama">Llama.cpp</option>
      <option value="ollama">Ollama</option>
    </select>
  </div>
  <div class="filter-group">
    <label>
      <input type="checkbox" bind:checked={showFavoritesOnly} />
      Favorites Only
    </label>
  </div>
  <div class="filter-group">
    <Input
      type="text"
      placeholder="Search models..."
      bind:value={searchQuery}
    />
  </div>
</div>

<style>
  .filters {
    display: flex;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .filter-group label {
    font-size: 0.9rem;
    color: var(--text-secondary, #666);
  }

  .filter-group select {
    padding: 0.5rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 4px;
    background: var(--bg-primary, #fff);
    color: var(--text-primary, #100f0f);
  }

  @media screen and (max-width: 768px) {
    .filters {
      flex-direction: column;
      align-items: stretch;
    }

    .filter-group {
      width: 100%;
    }
  }
</style>

