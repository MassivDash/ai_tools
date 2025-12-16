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
        const key = getModelKey(note.platform, note.model_name)
        newNotes.set(key, note)
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
    modelPath?: string
  ) => {
    const note = getNote(platform, modelName)
    editingNote = {
      platform,
      model_name: modelName,
      model_path: modelPath,
      is_favorite: note?.is_favorite || false,
      tags: note?.tags || [],
      notes: note?.notes || ''
    }
    editingTags = editingNote.tags.join(', ')
    editingNotes = editingNote.notes || ''
  }

  const saveNote = async () => {
    if (!editingNote) return

    const tags = editingTags
      .split(',')
      .map((t) => t.trim())
      .filter((t) => t.length > 0)

    const noteRequest: ModelNoteRequest = {
      platform: editingNote.platform,
      model_name: editingNote.model_name,
      model_path: editingNote.model_path,
      is_favorite: editingNote.is_favorite,
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
    } catch (err: any) {
      console.error('❌ Failed to save note:', err)
      error = err.response?.data?.error || err.message || 'Failed to save note'
    }
  }

  const cancelEditing = () => {
    editingNote = null
    editingTags = ''
    editingNotes = ''
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
  <PageSubHeader title="Model Notes">
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
      {getNote}
      {isFavorite}
      {getTags}
      {getNotes}
      {toggleFavorite}
      {startEditing}
      {deleteNote}
      {modelNotesKey}
    />

    <PlatformSection
      title="Ollama Models"
      icon="database"
      models={filteredOllamaModels()}
      platform="ollama"
      {getNote}
      {isFavorite}
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
    border-radius: 4px;
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
