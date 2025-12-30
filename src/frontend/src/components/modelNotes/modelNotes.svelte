<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import ModelFilters from './ModelFilters.svelte'
  import ModelEditModal from './ModelEditModal.svelte'
  import PlatformSection from './PlatformSection.svelte'
  import PageSubHeader from '../ui/PageSubHeader.svelte'
  import type {
    LlamaModelInfo,
    OllamaModelInfo,
    ModelNote,
    ModelNoteRequest
  } from './types'

  let llamaModels: LlamaModelInfo[] = $state([])
  let ollamaModels: OllamaModelInfo[] = $state([])
  let modelNotesData: Map<string, ModelNote> = $state(new Map())
  let modelNotesKey = $state(0)
  let loading = $state(false)
  let error = $state('')
  let selectedPlatform: 'llama' | 'ollama' | 'all' = $state('all')
  let showFavoritesOnly = $state(false)
  let searchQuery = $state('')
  let minSize = $state(0)
  let maxSize = $state(100)
  let editingNote: ModelNote | null = $state(null)
  let editingTags = $state('')
  let editingNotes = $state('')
  let editingIsDefault = $state(false)

  // Ensure size filter values are always valid numbers
  $effect(() => {
    // Only fix invalid values, don't change valid ones
    if (typeof minSize !== 'number' || isNaN(minSize)) {
      minSize = 0
    } else if (minSize < 0) {
      minSize = 0
    }

    if (typeof maxSize !== 'number' || isNaN(maxSize)) {
      maxSize = 100
    } else if (maxSize > 100) {
      maxSize = 100
    }

    // Ensure minSize doesn't exceed maxSize
    if (minSize > maxSize) {
      const temp = minSize
      minSize = Math.max(0, maxSize)
      maxSize = Math.min(100, temp)
    }
  })

  const getModelKey = (platform: string, modelName: string): string => {
    return `${platform}:${modelName}`
  }

  const getNote = (platform: string, modelName: string): ModelNote | null => {
    modelNotesKey
    const key = getModelKey(platform, modelName)
    return modelNotesData.get(key) || null
  }

  const isFavorite = (platform: string, modelName: string): boolean => {
    modelNotesKey
    const note = getNote(platform, modelName)
    return note?.is_favorite || false
  }

  const getTags = (platform: string, modelName: string): string[] => {
    modelNotesKey
    const note = getNote(platform, modelName)
    return note?.tags || []
  }

  const getNotes = (platform: string, modelName: string): string => {
    modelNotesKey
    const note = getNote(platform, modelName)
    return note?.notes || ''
  }

  const isDefault = (platform: string, modelName: string): boolean => {
    modelNotesKey
    const note = getNote(platform, modelName)
    return note?.is_default || false
  }

  // Normalize size to bytes
  const normalizeSizeToBytes = (size: number | string | undefined): number => {
    if (!size) return 0

    // If it's already a number (llama.cpp), return it
    if (typeof size === 'number') {
      return size
    }

    // If it's a string (ollama), parse it (e.g., "4.7 GB", "657 MB")
    const sizeStr = size.trim().toUpperCase()
    const match = sizeStr.match(/^([\d.]+)\s*(B|KB|MB|GB|TB)$/)
    if (!match) return 0

    const value = parseFloat(match[1])
    const unit = match[2]

    const multipliers: Record<string, number> = {
      B: 1,
      KB: 1024,
      MB: 1024 * 1024,
      GB: 1024 * 1024 * 1024,
      TB: 1024 * 1024 * 1024 * 1024
    }

    return value * (multipliers[unit] || 1)
  }

  const loadModels = async () => {
    loading = true
    error = ''

    try {
      const llamaResponse = await axiosBackendInstance.get<{
        local_models: LlamaModelInfo[]
      }>('llama-server/models')
      llamaModels = llamaResponse.data.local_models

      const ollamaResponse = await axiosBackendInstance.get<{
        models: OllamaModelInfo[]
      }>('chromadb/models')
      ollamaModels = ollamaResponse.data.models
    } catch (err: any) {
      console.error('❌ Failed to load models:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to load models'
    } finally {
      loading = false
    }
  }

  const loadModelNotes = async () => {
    try {
      const response = await axiosBackendInstance.get<{ notes: ModelNote[] }>(
        'model-notes'
      )
      const newNotes = new Map<string, ModelNote>()
      for (const note of response.data.notes) {
        // Store by model_name (which is hf_format for llama defaults, or filename for others)
        const key = getModelKey(note.platform, note.model_name)
        newNotes.set(key, note)
        // Also store by filename for llama models if we have both (for backward compatibility)
        if (note.platform === 'llama' && note.model_path) {
          // Extract filename from path for lookup
          const filename =
            note.model_path.split(/[/\\]/).pop() || note.model_name
          if (filename !== note.model_name) {
            const filenameKey = getModelKey(note.platform, filename)
            // Only set if not already set (prefer hf_format key)
            if (!newNotes.has(filenameKey)) {
              newNotes.set(filenameKey, note)
            }
          }
        }
      }
      modelNotesData = newNotes
      modelNotesKey++
    } catch (err: any) {
      console.error('❌ Failed to load model notes:', err)
    }
  }

  const toggleFavorite = async (
    platform: string,
    modelName: string,
    modelPath?: string
  ) => {
    const currentNote = getNote(platform, modelName)
    const isCurrentlyFavorite = currentNote?.is_favorite || false

    const noteRequest: ModelNoteRequest = {
      platform,
      model_name: modelName,
      model_path: modelPath,
      is_favorite: !isCurrentlyFavorite,
      tags: currentNote?.tags || [],
      notes: currentNote?.notes || undefined
    }

    try {
      const response = await axiosBackendInstance.post<{ note: ModelNote }>(
        'model-notes',
        noteRequest
      )
      const key = getModelKey(platform, modelName)
      modelNotesData.set(key, response.data.note)
      modelNotesData = new Map(modelNotesData)
      modelNotesKey++
      error = ''
    } catch (err: any) {
      console.error('❌ Failed to toggle favorite:', err)
      const errorMsg =
        err.response?.data?.error || err.message || 'Failed to update favorite'
      error = errorMsg
    }
  }

  const startEditing = (
    platform: string,
    modelName: string,
    modelPath?: string,
    hfFormat?: string
  ) => {
    // For llama models, check if there's a note by hf_format first
    let note = null
    if (platform === 'llama' && hfFormat) {
      note = getNote(platform, hfFormat)
    }
    if (!note) {
      note = getNote(platform, modelName)
    }

    // For llama default models, use hf_format as model_name
    // For non-default or ollama, use the filename/model name
    const storedModelName =
      note?.is_default && platform === 'llama' && hfFormat
        ? hfFormat
        : note?.model_name || modelName

    editingNote = {
      platform,
      model_name: storedModelName,
      model_path: modelPath,
      is_favorite: note?.is_favorite || false,
      is_default: note?.is_default || false,
      tags: note?.tags || [],
      notes: note?.notes || ''
    }
    editingTags = editingNote.tags.join(', ')
    editingNotes = editingNote.notes || ''
    editingIsDefault = editingNote.is_default
  }

  const saveNote = async () => {
    if (!editingNote) return

    const tags = editingTags
      .split(',')
      .map((t) => t.trim())
      .filter((t) => t.length > 0)

    // For llama default models, model_name should be in HuggingFace format (user/model:quant)
    // For ollama, model_name is just the model name
    // For non-default models, we can keep model_path for reference
    // For llama default models, ensure we use hf_format if available
    let finalModelName = editingNote.model_name
    if (editingIsDefault && editingNote.platform === 'llama') {
      // Find the model to get its hf_format
      const model = llamaModels.find(
        (m) =>
          m.name === editingNote.model_name ||
          m.hf_format === editingNote.model_name
      )
      if (model?.hf_format) {
        finalModelName = model.hf_format
      }
    }

    const noteRequest: ModelNoteRequest = {
      platform: editingNote.platform,
      model_name: finalModelName, // hf_format for llama defaults, name for others
      // For default models, don't store the path - just the name in HuggingFace format
      // Backend will handle downloading/caching automatically
      model_path: editingIsDefault ? undefined : editingNote.model_path,
      is_favorite: editingNote.is_favorite,
      is_default: editingIsDefault,
      tags,
      notes: editingNotes.trim() || undefined
    }

    const modelName = editingNote.model_name
    const platform = editingNote.platform

    try {
      const response = await axiosBackendInstance.post<{ note: ModelNote }>(
        'model-notes',
        noteRequest
      )
      const key = getModelKey(platform, modelName)
      modelNotesData.set(key, response.data.note)
      modelNotesData = new Map(modelNotesData)
      modelNotesKey++
      editingNote = null
      editingTags = ''
      editingNotes = ''
      editingIsDefault = false
    } catch (err: any) {
      console.error('❌ Failed to save note:', err)
      error = err.response?.data?.error || err.message || 'Failed to save note'
    }
  }

  const cancelEditing = () => {
    editingNote = null
    editingTags = ''
    editingNotes = ''
    editingIsDefault = false
  }

  const deleteNote = async (platform: string, modelName: string) => {
    if (!confirm(`Delete notes for ${modelName}?`)) return

    try {
      await axiosBackendInstance.delete(
        `model-notes/${platform}/${encodeURIComponent(modelName)}`
      )
      const key = getModelKey(platform, modelName)
      modelNotesData.delete(key)
      modelNotesData = new Map(modelNotesData)
      modelNotesKey++
    } catch (err: any) {
      console.error('❌ Failed to delete note:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to delete note'
    }
  }

  const filteredLlamaModels = (): LlamaModelInfo[] => {
    let filtered = llamaModels

    if (selectedPlatform !== 'all' && selectedPlatform !== 'llama') {
      return []
    }

    if (showFavoritesOnly) {
      filtered = filtered.filter((m) => isFavorite('llama', m.name))
    }

    if (searchQuery) {
      const query = searchQuery.toLowerCase()
      filtered = filtered.filter(
        (m) =>
          m.name.toLowerCase().includes(query) ||
          m.hf_format?.toLowerCase().includes(query)
      )
    }

    // Filter by size (convert to GB for comparison)
    // Default to showing all models (0-100GB range)
    const currentMin =
      typeof minSize === 'number' && !isNaN(minSize) ? minSize : 0
    const currentMax =
      typeof maxSize === 'number' && !isNaN(maxSize) ? maxSize : 100

    // Only apply size filter if NOT at full range (0-100)
    // This ensures all models show by default
    if (currentMin !== 0 || currentMax !== 100) {
      filtered = filtered.filter((m) => {
        // Always include models without size information
        if (!m.size) return true

        const sizeBytes = normalizeSizeToBytes(m.size)
        // Include models with invalid/zero size
        if (sizeBytes === 0) return true

        const sizeGB = sizeBytes / (1024 * 1024 * 1024)
        // Filter by size range
        return sizeGB >= currentMin && sizeGB <= currentMax
      })
    }
    // If at default (0-100), don't filter - show all models

    return filtered
  }

  const filteredOllamaModels = (): OllamaModelInfo[] => {
    let filtered = ollamaModels

    if (selectedPlatform !== 'all' && selectedPlatform !== 'ollama') {
      return []
    }

    if (showFavoritesOnly) {
      filtered = filtered.filter((m) => isFavorite('ollama', m.name))
    }

    if (searchQuery) {
      const query = searchQuery.toLowerCase()
      filtered = filtered.filter((m) => m.name.toLowerCase().includes(query))
    }

    // Filter by size (convert to GB for comparison)
    // Default to showing all models (0-100GB range)
    const currentMin =
      typeof minSize === 'number' && !isNaN(minSize) ? minSize : 0
    const currentMax =
      typeof maxSize === 'number' && !isNaN(maxSize) ? maxSize : 100

    // Only apply size filter if NOT at full range (0-100)
    // This ensures all models show by default
    if (currentMin !== 0 || currentMax !== 100) {
      filtered = filtered.filter((m) => {
        // Always include models without size information
        if (!m.size) return true

        const sizeBytes = normalizeSizeToBytes(m.size)
        // Include models with invalid/zero size
        if (sizeBytes === 0) return true

        const sizeGB = sizeBytes / (1024 * 1024 * 1024)
        // Filter by size range
        return sizeGB >= currentMin && sizeGB <= currentMax
      })
    }
    // If at default (0-100), don't filter - show all models

    return filtered
  }

  onMount(() => {
    loadModels()
    loadModelNotes()
  })
</script>

<div class="model-notes">
  <PageSubHeader title="Model Notes" icon="note">
    {#snippet actions()}
      <Button variant="info" onclick={loadModels} disabled={loading}>
        <MaterialIcon name="refresh" width="20" height="20" />
        Refresh Models
      </Button>
    {/snippet}
  </PageSubHeader>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <ModelFilters
    bind:selectedPlatform
    bind:showFavoritesOnly
    bind:searchQuery
    bind:minSize
    bind:maxSize
  />

  {#if loading}
    <div class="loading">Loading models...</div>
  {:else}
    <PlatformSection
      title="Llama.cpp Models"
      icon="server-network"
      models={filteredLlamaModels()}
      platform="llama"
      getNote={(platform, modelName) => {
        // For llama, try to find note by hf_format first, then by filename
        const model = filteredLlamaModels().find((m) => m.name === modelName)
        if (model?.hf_format) {
          const hfNote = getNote(platform, model.hf_format)
          if (hfNote) return hfNote
        }
        return getNote(platform, modelName)
      }}
      isFavorite={(platform, modelName) => {
        const model = filteredLlamaModels().find((m) => m.name === modelName)
        if (model?.hf_format) {
          const hfNote = getNote(platform, model.hf_format)
          if (hfNote?.is_favorite) return true
        }
        return isFavorite(platform, modelName)
      }}
      isDefault={(platform, modelName) => {
        const model = filteredLlamaModels().find((m) => m.name === modelName)
        if (model?.hf_format) {
          const hfNote = getNote(platform, model.hf_format)
          if (hfNote?.is_default) return true
        }
        return isDefault(platform, modelName)
      }}
      getTags={(platform, modelName) => {
        const model = filteredLlamaModels().find((m) => m.name === modelName)
        if (model?.hf_format) {
          const hfNote = getNote(platform, model.hf_format)
          if (hfNote) return hfNote.tags
        }
        return getTags(platform, modelName)
      }}
      getNotes={(platform, modelName) => {
        const model = filteredLlamaModels().find((m) => m.name === modelName)
        if (model?.hf_format) {
          const hfNote = getNote(platform, model.hf_format)
          if (hfNote) return hfNote.notes || ''
        }
        return getNotes(platform, modelName)
      }}
      toggleFavorite={(platform, modelName, modelPath) => {
        const model = filteredLlamaModels().find((m) => m.name === modelName)
        const identifier = model?.hf_format || modelName
        toggleFavorite(platform, identifier, modelPath)
      }}
      startEditing={(platform, modelName, modelPath) => {
        const model = filteredLlamaModels().find((m) => m.name === modelName)
        // Pass hf_format if available
        const hfFormat = model?.hf_format
        startEditing(platform, modelName, modelPath, hfFormat)
      }}
      deleteNote={(platform, modelName) => {
        const model = filteredLlamaModels().find((m) => m.name === modelName)
        // Try both hf_format and filename
        if (model?.hf_format) {
          deleteNote(platform, model.hf_format)
        }
        deleteNote(platform, modelName)
      }}
      {modelNotesKey}
    />

    <PlatformSection
      title="Ollama Models"
      icon="database"
      models={filteredOllamaModels()}
      platform="ollama"
      {getNote}
      {isFavorite}
      {isDefault}
      {getTags}
      {getNotes}
      {toggleFavorite}
      {startEditing}
      {deleteNote}
      {modelNotesKey}
    />

    {#if filteredLlamaModels().length === 0 && filteredOllamaModels().length === 0}
      <div class="empty-state">
        <p>No models found matching your filters.</p>
      </div>
    {/if}
  {/if}

  {#if editingNote}
    <ModelEditModal
      note={editingNote}
      bind:tags={editingTags}
      bind:notes={editingNotes}
      bind:isFavorite={editingNote.is_favorite}
      bind:isDefault={editingIsDefault}
      onClose={cancelEditing}
      onSave={saveNote}
    />
  {/if}
</div>

<style>
  .model-notes {
    width: 100%;
    padding: 1rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .error {
    padding: 0.75rem;
    margin-bottom: 1rem;
    background-color: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 8px;
    color: var(--accent-color, #c33);
  }

  .loading {
    text-align: center;
    padding: 2rem;
    color: var(--text-secondary, #666);
  }

  .empty-state {
    text-align: center;
    padding: 3rem;
    color: var(--text-secondary, #666);
  }
</style>
