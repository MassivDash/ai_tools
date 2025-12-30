<script lang="ts">
  import Input from '../ui/Input.svelte'
  import CheckboxWithHelp from '../ui/CheckboxWithHelp.svelte'
  import DualRangeSlider from '../ui/DualRangeSlider.svelte'
  import Dropdown from '../ui/Dropdown.svelte'

  interface Props {
    selectedPlatform?: 'llama' | 'ollama' | 'all'
    showFavoritesOnly?: boolean
    searchQuery?: string
    minSize?: number
    maxSize?: number
  }

  let {
    selectedPlatform = $bindable('all'),
    showFavoritesOnly = $bindable(false),
    searchQuery = $bindable(''),
    minSize = $bindable(0),
    maxSize = $bindable(100)
  }: Props = $props()
</script>

<div class="filters">
  <div class="filter-row">
    <div class="filter-group">
      <Dropdown
        id="platform-select"
        label="Platform:"
        row
        bind:value={selectedPlatform}
        options={[
          { value: 'all', label: 'All Platforms' },
          { value: 'llama', label: 'Llama.cpp' },
          { value: 'ollama', label: 'Ollama' }
        ]}
      />
    </div>
    <div class="filter-group">
      <CheckboxWithHelp
        bind:checked={showFavoritesOnly}
        label="Favorites Only"
      />
    </div>
  </div>
  <div class="filter-row filter-row-search">
    <div class="filter-group search-group">
      <Input
        type="text"
        placeholder="Search models..."
        bind:value={searchQuery}
      />
    </div>
    <div class="filter-group slider-group">
      <span class="slider-label">Size (GB):</span>
      <DualRangeSlider
        min={0}
        max={100}
        step={1}
        bind:minValue={minSize}
        bind:maxValue={maxSize}
        tickInterval={10}
      />
    </div>
  </div>
</div>

<style>
  .filters {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .filter-row {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .filter-row-search {
    align-items: flex-end;
    width: 100%;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .search-group {
    width: 30%;
    min-width: 200px;
  }

  .slider-group {
    flex: 1;
    min-width: 400px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .slider-group :global(.dual-range-slider) {
    width: 100%;
    display: block;
  }

  .slider-label {
    font-size: 0.9rem;
    color: var(--text-secondary, #666);
  }

  @media screen and (max-width: 768px) {
    .filter-row {
      flex-direction: column;
      align-items: stretch;
    }

    .search-group,
    .slider-group {
      width: 100%;
      min-width: unset;
    }
  }
</style>
