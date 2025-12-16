<script lang="ts">
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import type { ModelNote } from './types'

  interface Props {
    model: {
      name: string
      path?: string
      size?: number | string
      hf_format?: string
      modified?: string
    }
    platform: 'llama' | 'ollama'
    note: ModelNote | null
    isFavorite: boolean
    tags: string[]
    notes: string
    onToggleFavorite: () => void
    onEdit: () => void
    onDelete: () => void
  }

  let {
    model,
    platform,
    note,
    isFavorite,
    tags,
    notes,
    onToggleFavorite,
    onEdit,
    onDelete
  }: Props = $props()

  const formatFileSize = (bytes?: number): string => {
    if (!bytes) return 'Unknown size'
    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    let size = bytes
    let unitIndex = 0
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024
      unitIndex++
    }
    return `${size.toFixed(2)} ${units[unitIndex]}`
  }
</script>

<div class="model-card" class:favorite={isFavorite}>
  <div class="model-header">
    <div class="model-name">
      {#if isFavorite}
        <MaterialIcon name="star" width="20" height="20" class="star-icon" />
      {/if}
      <span>{model.name}</span>
    </div>
    <div class="model-actions">
      <Button
        variant={isFavorite ? 'primary' : 'info'}
        class="button-icon-only"
        onclick={onToggleFavorite}
        title={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
      >
        <MaterialIcon
          name={isFavorite ? 'star' : 'star-outline'}
          width="20"
          height="20"
        />
      </Button>
      <Button
        variant="info"
        class="button-icon-only"
        onclick={onEdit}
        title="Edit notes"
      >
        <MaterialIcon name="pencil" width="20" height="20" />
      </Button>
      {#if note}
        <Button
          variant="danger"
          class="button-icon-only"
          onclick={onDelete}
          title="Delete notes"
        >
          <MaterialIcon name="delete" width="20" height="20" />
        </Button>
      {/if}
    </div>
  </div>
  <div class="model-info">
    {#if platform === 'llama'}
      <div class="info-row">
        <span class="label">Size:</span>
        <span>{formatFileSize(model.size as number)}</span>
      </div>
      {#if model.hf_format}
        <div class="info-row">
          <span class="label">HF Format:</span>
          <span>{model.hf_format}</span>
        </div>
      {/if}
    {:else}
      {#if model.size}
        <div class="info-row">
          <span class="label">Size:</span>
          <span>{model.size}</span>
        </div>
      {/if}
      {#if model.modified}
        <div class="info-row">
          <span class="label">Modified:</span>
          <span>{model.modified}</span>
        </div>
      {/if}
    {/if}
    {#if tags.length > 0}
      <div class="tags">
        {#each tags as tag}
          <span class="tag">{tag}</span>
        {/each}
      </div>
    {/if}
    {#if notes}
      <div class="notes-preview">{notes}</div>
    {/if}
  </div>
</div>

<style>
  .model-card {
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    padding: 1rem;
    background: var(--bg-primary, #fff);
    transition: all 0.2s ease;
  }

  .model-card:hover {
    box-shadow: 0 2px 8px var(--shadow, rgba(0, 0, 0, 0.1));
  }

  .model-card.favorite {
    border-color: var(--accent-color, #b12424);
    background: var(--bg-tertiary, #ffdad6);
  }

  .model-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .model-name {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 600;
    color: var(--text-primary, #100f0f);
    flex: 1;
    min-width: 0;
  }

  .model-name span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-name :global(.star-icon) {
    color: var(--accent-color, #b12424);
    flex-shrink: 0;
  }

  .model-actions {
    display: flex;
    gap: 0.25rem;
  }

  .model-actions :global(.button-icon-only) {
    padding: 0.5rem !important;
    min-width: 2.5rem !important;
    min-height: 2.5rem !important;
  }

  .model-info {
    font-size: 0.9rem;
    color: var(--text-secondary, #666);
  }

  .info-row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .info-row .label {
    font-weight: 600;
    min-width: 60px;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.5rem;
  }

  .tag {
    padding: 0.25rem 0.5rem;
    background: var(--bg-secondary, #f5ddd9);
    border-radius: 4px;
    font-size: 0.8rem;
    color: var(--text-primary, #100f0f);
  }

  .notes-preview {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background: var(--bg-secondary, #f5ddd9);
    border-radius: 4px;
    font-size: 0.85rem;
    max-height: 100px;
    overflow-y: auto;
  }
</style>
