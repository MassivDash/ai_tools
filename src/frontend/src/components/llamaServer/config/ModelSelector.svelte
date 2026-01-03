<script lang="ts">
  import SearchableList from '../../ui/SearchableList.svelte'
  import type { ModelNote } from '@types'
  import { formatFileSize, getDisplayValue } from './utils'

  interface ModelInfo {
    name: string
    path: string
    size?: number
    hf_format?: string
  }

  interface Props {
    loadingModels: boolean
    localModels: ModelInfo[]
    modelNotes: Map<string, ModelNote>
    newHfModel: string
    newHfModelBackend: string
    onSelect: (_model: ModelInfo) => void
  }

  let {
    loadingModels,
    localModels,
    modelNotes,
    newHfModel,
    newHfModelBackend,
    onSelect
  }: Props = $props()

  // Use a combination to ensure uniqueness - path should be unique, but add name as fallback
  const getModelKey = (model: ModelInfo, index: number) => {
    // Use path as primary key (should be unique), fallback to name + index if path is missing
    if (model.path) return model.path
    if (model.name) return `${model.name}-${index}`
    if (model.hf_format) return `${model.hf_format}-${index}`
    return `model-${index}`
  }
  const getModelLabel = (model: ModelInfo) => model.name
  const getModelSubtext = (model: ModelInfo) => {
    const parts = []
    if (model.hf_format) {
      parts.push(model.hf_format)
    } else if (model.path) {
      parts.push(model.path)
    }
    if (model.size) {
      parts.push(formatFileSize(model.size))
    }
    return parts.join(' • ')
  }

  // Find matching note for a model
  const getModelNote = (model: ModelInfo): ModelNote | null => {
    // Try matching by name first
    let note = modelNotes.get(model.name)
    if (note) return note

    // Try matching by path
    if (model.path) {
      note = modelNotes.get(model.path)
      if (note) return note
    }

    // Try matching by hf_format
    if (model.hf_format) {
      note = modelNotes.get(model.hf_format)
      if (note) return note
    }

    return null
  }

  const getModelFavorite = (model: ModelInfo): boolean => {
    const note = getModelNote(model)
    return note?.is_favorite || false
  }

  const getModelTags = (model: ModelInfo): string[] => {
    const note = getModelNote(model)
    return note?.tags || []
  }

  const getModelNotes = (model: ModelInfo): string => {
    const note = getModelNote(model)
    if (!note?.notes) return ''
    // Return a preview (first 100 chars)
    return note.notes.length > 100
      ? note.notes.substring(0, 100) + '...'
      : note.notes
  }
</script>

<div class="config-section">
  <div class="section-label">Local GGUF Models:</div>
  {#if loadingModels}
    <div class="loading-models">Loading models...</div>
  {:else if localModels.length > 0}
    <SearchableList
      items={localModels}
      searchPlaceholder="Search models..."
      emptyMessage="No models found"
      getItemKey={getModelKey}
      getItemLabel={getModelLabel}
      getItemSubtext={getModelSubtext}
      getItemFavorite={getModelFavorite}
      getItemTags={getModelTags}
      getItemNotes={getModelNotes}
      selectedKey={(() => {
        // Match by name (filename) or by backend value (path/hf_format)
        const selected = localModels.find(
          (m) =>
            m.name === newHfModel ||
            m.path === newHfModelBackend ||
            m.hf_format === newHfModelBackend ||
            m.path === newHfModelBackend ||
            m.hf_format === newHfModelBackend ||
            getDisplayValue(m.path) === newHfModel ||
            getDisplayValue(newHfModelBackend) === m.name
        )
        if (!selected) return null
        // Use path as key (should be unique), fallback to name
        return selected.path || selected.name || selected.hf_format || null
      })()}
      onselect={onSelect}
    />
  {:else}
    <div class="no-models">
      <p>No GGUF models found in ~/.cache/llama.cpp/</p>
      <p class="hint-small">Models will appear here once downloaded</p>
    </div>
  {/if}
</div>

<style>
  .config-section {
    margin-bottom: 2rem;
  }

  .section-label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
    color: var(--text-primary, #333);
    transition: color 0.3s ease;
  }

  .loading-models {
    padding: 1rem;
    text-align: center;
    color: var(--text-secondary, #666);
    transition: color 0.3s ease;
  }

  .no-models {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary, #666);
    transition: color 0.3s ease;
  }

  .no-models .hint-small {
    font-size: 0.85rem;
    color: var(--text-tertiary, #999);
    margin-top: 0.5rem;
    transition: color 0.3s ease;
  }
</style>
