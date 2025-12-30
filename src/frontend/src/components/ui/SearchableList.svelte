<script lang="ts">
  import MaterialIcon from './MaterialIcon.svelte'
  import Input from './Input.svelte'
  import IconButton from './IconButton.svelte'

  export let items: any[] = []
  export let searchPlaceholder: string = 'Search...'
  export let emptyMessage: string = 'No items found'
  export let getItemKey: (_item: any, _index: number) => string = (
    _item,
    _index
  ) => String(_item.id || _item.name || _item || _index)
  export let getItemLabel: (_item: any) => string = (_item) =>
    String(_item.name || _item.label || _item)
  export let getItemSubtext: ((_item: any) => string) | undefined = undefined
  export let getItemFavorite: ((_item: any) => boolean) | undefined = undefined
  export let getItemTags: ((_item: any) => string[]) | undefined = undefined
  export let getItemNotes: ((_item: any) => string) | undefined = undefined
  export let selectedKey: string | null = null
  export let maxHeight: string = '300px'
  export let onselect: ((_item: any) => void) | undefined = undefined

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
    onselect?.(item)
  }
</script>

<div class="searchable-list">
  <div class="search-wrapper">
    <Input
      type="text"
      placeholder={searchPlaceholder}
      bind:value={searchQuery}
      class="input search-input"
    />
    {#if searchQuery}
      <IconButton
        variant="ghost"
        size="small"
        class="clear-search"
        onclick={() => (searchQuery = '')}
        aria-label="Clear search"
        iconSize="16"
      >
        <span>×</span>
      </IconButton>
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
            <div class="item-header">
              <div class="item-label">
                {#if getItemFavorite && getItemFavorite(item)}
                  <MaterialIcon
                    name="star"
                    width="16"
                    height="16"
                    class="favorite-icon"
                  />
                {/if}
                <span>{getItemLabel(item)}</span>
              </div>
            </div>
            {#if getItemSubtext !== undefined}
              <div class="item-subtext">{getItemSubtext(item)}</div>
            {/if}
            {#if getItemTags && getItemTags(item).length > 0}
              <div class="item-tags">
                {#each getItemTags(item) as tag}
                  <span class="tag">{tag}</span>
                {/each}
              </div>
            {/if}
            {#if getItemNotes && getItemNotes(item)}
              <div class="item-notes">{getItemNotes(item)}</div>
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

  /* Target the input element inside Input component if possible or via the passed class */
  :global(.search-input) {
    padding-right: 2rem !important;
  }

  /* :global(.search-input:focus) handled by Input component */

  :global(.clear-search) {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s;
    border-radius: 8px;
    z-index: 2; /* Ensure above input */
    min-width: 1.5rem !important;
    min-height: 1.5rem !important;
    width: 1.5rem !important;
    height: 1.5rem !important;
    padding: 0 !important;
  }

  /* Override styles for the icon inside clear button */
  :global(.clear-search span) {
    font-size: 1.2rem;
    line-height: 1;
    margin-top: -2px; /* Center alignment tweak */
  }

  .list-container {
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    overflow-y: auto;
    background-color: var(--bg-primary, white);
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease;
  }

  .list {
    display: flex;
    flex-direction: column;
  }

  .list-item {
    width: 100%;
    padding: 1rem;
    border: none;
    border-bottom: 1px solid var(--border-color, #f0f0f0);
    background: var(--bg-primary, white);
    text-align: left;
    cursor: pointer;
    transition:
      background-color 0.2s,
      border-color 0.3s ease;
    color: var(--text-primary, #333);
  }

  .list-item:hover {
    background-color: var(--bg-secondary, #f5f5f5);
  }

  .list-item.selected {
    background-color: rgba(33, 150, 243, 0.1);
    border-left: 3px solid var(--accent-color, #2196f3);
  }

  .list-item:last-child {
    border-bottom: none;
  }

  .item-header {
    margin-bottom: 0.25rem;
  }

  .item-label {
    font-weight: 600;
    color: var(--text-primary, #333);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transition: color 0.3s ease;
  }

  .item-label :global(.favorite-icon) {
    color: var(--accent-color, #b12424);
    flex-shrink: 0;
  }

  .item-subtext {
    font-size: 0.85rem;
    color: var(--text-secondary, #666);
    display: flex;
    justify-content: space-between;
    align-items: center;
    transition: color 0.3s ease;
    margin-top: 0.25rem;
  }

  .item-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.5rem;
  }

  .tag {
    padding: 0.25rem 0.5rem;
    background: var(--bg-secondary, #f5ddd9);
    border-radius: 8px;
    font-size: 0.8rem;
    color: var(--text-primary, #100f0f);
    transition:
      background-color 0.3s ease,
      color 0.3s ease;
  }

  .item-notes {
    font-size: 0.8rem;
    color: var(--text-secondary, #666);
    margin-top: 0.5rem;
    padding: 0.5rem;
    background: var(--bg-secondary, #f5f5f5);
    border-radius: 8px;
    max-height: 3rem;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    transition:
      color 0.3s ease,
      background-color 0.3s ease;
  }

  .empty-message {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary, #666);
    font-style: italic;
    transition: color 0.3s ease;
  }
</style>
