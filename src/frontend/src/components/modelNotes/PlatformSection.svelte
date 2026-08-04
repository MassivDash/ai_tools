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
    deleteNote: (_platform: string, _modelNames: string[]) => void
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
      {#each models as model, index (model.path || `${model.name}-${index}`)}
        {#key `${model.path || model.name}-${modelNotesKey}`}
          {@const modelPath = platform === 'llama' ? model.path : undefined}
          {@const hfFormat = platform === 'llama' ? model.hf_format : undefined}
          {@const identifier = modelPath || model.name}
          {@const note = getNote(platform, identifier)}
          {@const isFav = isFavorite(platform, identifier)}
          {@const isDef = isDefault(platform, identifier)}
          {@const tags = getTags(platform, identifier)}
          {@const notes = getNotes(platform, identifier)}
          <ModelCard
            {model}
            {platform}
            {note}
            isFavorite={isFav}
            isDefault={isDef}
            {tags}
            {notes}
            onToggleFavorite={() => {
              toggleFavorite(platform, identifier, modelPath)
            }}
            onEdit={() => {
              startEditing(platform, identifier, modelPath, hfFormat)
            }}
            onDelete={() => {
              // Try every identifier a note for this model could have been
              // saved under (current path-based keying, and the older
              // hf_format/name keying) so deleting still works either way -
              // deleteNote confirms once, then tries each in turn.
              const candidates = [modelPath, hfFormat, model.name].filter(
                (value): value is string => Boolean(value)
              )
              deleteNote(platform, candidates)
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
