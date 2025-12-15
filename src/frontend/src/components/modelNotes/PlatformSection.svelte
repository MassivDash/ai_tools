<script lang="ts">
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import ModelCard from './ModelCard.svelte'
  import type { ModelNote } from './types'

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
    getNote: (platform: string, modelName: string) => ModelNote | null
    isFavorite: (platform: string, modelName: string) => boolean
    getTags: (platform: string, modelName: string) => string[]
    getNotes: (platform: string, modelName: string) => string
    toggleFavorite: (platform: string, modelName: string, modelPath?: string) => void
    startEditing: (platform: string, modelName: string, modelPath?: string) => void
    deleteNote: (platform: string, modelName: string) => void
    modelNotesKey: number
  }

  let {
    title,
    icon,
    models,
    platform,
    getNote,
    isFavorite,
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
          {@const tags = getTags(platform, model.name)}
          {@const notes = getNotes(platform, model.name)}
          <ModelCard
            {model}
            {platform}
            {note}
            isFavorite={isFav}
            {tags}
            {notes}
            onToggleFavorite={() => toggleFavorite(platform, model.name, model.path)}
            onEdit={() => startEditing(platform, model.name, model.path)}
            onDelete={() => deleteNote(platform, model.name)}
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

