<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  
  export let items: any[] = []
  export let searchPlaceholder: string = 'Search...'
  export let emptyMessage: string = 'No items found'
  export let getItemKey: (item: any, index: number) => string = (item, index) => String(item.id || item.name || item || index)
  export let getItemLabel: (item: any) => string = (item) => String(item.name || item.label || item)
  export let getItemSubtext: ((item: any) => string) | undefined = undefined
  export let selectedKey: string | null = null
  export let maxHeight: string = '300px'
  
  const dispatch = createEventDispatcher()
  
  let searchQuery = ''
  
  $: filteredItems = items.filter((item) => {
    if (!searchQuery.trim()) return true
    const query = searchQuery.toLowerCase()
    const label = getItemLabel(item).toLowerCase()
    if (getItemSubtext) {
      const subtext = getItemSubtext(item).toLowerCase()
      return label.includes(query) || subtext.includes(query)
    }
    return label.includes(query)
  })
  
  function handleItemClick(item: any) {
    dispatch('select', item)
  }
</script>

<div class="searchable-list">
  <div class="search-wrapper">
    <input
      type="text"
      class="search-input"
      placeholder={searchPlaceholder}
      bind:value={searchQuery}
    />
    {#if searchQuery}
      <button
        class="clear-search"
        onclick={() => (searchQuery = '')}
        aria-label="Clear search"
      >
        ×
      </button>
    {/if}
  </div>
  
  <div class="list-container" style="max-height: {maxHeight}">
    {#if filteredItems.length === 0}
      <div class="empty-message">{emptyMessage}</div>
    {:else}
      <div class="list" role="list">
        {#each filteredItems as item, index (getItemKey(item, index))}
          <button
            type="button"
            class="list-item"
            class:selected={selectedKey === getItemKey(item, index)}
            onclick={() => handleItemClick(item)}
          >
            <div class="item-label">{getItemLabel(item)}</div>
            {#if getItemSubtext !== undefined}
              <div class="item-subtext">{getItemSubtext(item)}</div>
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .searchable-list {
    display: flex;
    flex-direction: column;
  }

  .search-wrapper {
    position: relative;
    margin-bottom: 0.5rem;
  }

  .search-input {
    width: 100%;
    padding: 0.5rem;
    padding-right: 2rem;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 0.9rem;
    box-sizing: border-box;
  }

  .search-input:focus {
    outline: none;
    border-color: #2196f3;
  }

  .clear-search {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    font-size: 1.2rem;
    cursor: pointer;
    color: #999;
    padding: 0;
    width: 1.5rem;
    height: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .clear-search:hover {
    color: #333;
  }

  .list-container {
    border: 1px solid #ddd;
    border-radius: 4px;
    overflow-y: auto;
  }

  .list {
    display: flex;
    flex-direction: column;
  }

  .list-item {
    width: 100%;
    padding: 1rem;
    border: none;
    border-bottom: 1px solid #f0f0f0;
    background: white;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .list-item:hover {
    background-color: #f5f5f5;
  }

  .list-item.selected {
    background-color: #e3f2fd;
    border-left: 3px solid #2196f3;
  }

  .list-item:last-child {
    border-bottom: none;
  }

  .item-label {
    font-weight: 600;
    color: #333;
    margin-bottom: 0.25rem;
  }

  .item-subtext {
    font-size: 0.85rem;
    color: #666;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .empty-message {
    padding: 2rem;
    text-align: center;
    color: #666;
    font-style: italic;
  }
</style>

