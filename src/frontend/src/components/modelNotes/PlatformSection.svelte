<script lang="ts">
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import ModelCard from './ModelCard.svelte'
  import type { ModelNote } from '@types'

  interface Model {
    name: string
    path?: string
    size?: number | string
    hf_format?: string
    modified?: string
  }

  interface Props {
    title: string
    icon: string
    models: Model[]
    platform: 'llama' | 'ollama'
    getNote: (_platform: string, _modelName: string) => ModelNote | null
    isFavorite: (_platform: string, _modelName: string) => boolean
    isDefault: (_platform: string, _modelName: string) => boolean
    getTags: (_platform: string, _modelName: string) => string[]
    getNotes: (_platform: string, _modelName: string) => string
    toggleFavorite: (
      _platform: string,
      _modelName: string,
      _modelPath?: string
    ) => void
    startEditing: (
      _platform: string,
      _modelName: string,
      _modelPath?: string,
      _hfFormat?: string
    ) => void
    deleteNote: (_platform: string, _modelName: string) => void
    modelNotesKey: number
  }

  let {
    title,
    icon,
    models,
    platform,
    getNote,
    isFavorite,
    isDefault,
    getTags,
    getNotes,
    toggleFavorite,
    startEditing,
    deleteNote,
    modelNotesKey
  }: Props = $props()
</script>

{#if models.length > 0}
  <div class="platform-section">
    <h3 class="platform-header">
      <MaterialIcon name={icon} width="24" height="24" />
      {title} ({models.length})
    </h3>
    <div class="models-grid">
      {#each models as model (model.name)}
        {#key `${model.name}-${modelNotesKey}`}
          {@const note = getNote(platform, model.name)}
          {@const isFav = isFavorite(platform, model.name)}
          {@const isDef = isDefault(platform, model.name)}
          {@const tags = getTags(platform, model.name)}
          {@const notes = getNotes(platform, model.name)}
          <ModelCard
            {model}
            {platform}
            {note}
            isFavorite={isFav}
            isDefault={isDef}
            {tags}
            {notes}
            onToggleFavorite={() => {
              // For llama, use hf_format if available; for ollama, just use name
              const identifier =
                platform === 'llama' && model.hf_format
                  ? model.hf_format
                  : model.name
              // Only pass path for llama models (ollama models don't have path)
              const modelPath = platform === 'llama' ? model.path : undefined
              toggleFavorite(platform, identifier, modelPath)
            }}
            onEdit={() => {
              // For llama models, pass hf_format if available; for ollama, just use name
              const identifier =
                platform === 'llama' && model.hf_format
                  ? model.hf_format
                  : model.name
              // Only pass path for llama models (ollama models don't have path)
              const modelPath = platform === 'llama' ? model.path : undefined
              const hfFormat =
                platform === 'llama' ? model.hf_format : undefined
              startEditing(platform, identifier, modelPath, hfFormat)
            }}
            onDelete={() => {
              // For llama, try both hf_format and name; for ollama, just use name
              if (platform === 'llama' && model.hf_format) {
                deleteNote(platform, model.hf_format)
              }
              deleteNote(platform, model.name)
            }}
          />
        {/key}
      {/each}
    </div>
  </div>
{/if}

<style>
  .platform-section {
    margin-bottom: 2rem;
  }

  .platform-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    padding-bottom: 0.5rem;
    border-bottom: 2px solid var(--border-color, #ddd);
    color: var(--text-primary, #100f0f);
    font-size: 1.25rem;
  }

  .platform-header :global(svg) {
    color: var(--accent-color, #b12424);
  }

  .models-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
    gap: 1rem;
  }

  @media screen and (max-width: 768px) {
    .models-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
