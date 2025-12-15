<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import ModelFilters from './ModelFilters.svelte'
  import ModelEditModal from './ModelEditModal.svelte'
  import PlatformSection from './PlatformSection.svelte'
  import type {
    LlamaModelInfo,
    OllamaModelInfo,
    ModelNote,
    ModelNoteRequest
  } from './types'

  let llamaModels: LlamaModelInfo[] = []
  let ollamaModels: OllamaModelInfo[] = []
  let modelNotes: Map<string, ModelNote> = new Map()
  let modelNotesKey = 0
  let loading = false
  let error = ''
  let selectedPlatform: 'llama' | 'ollama' | 'all' = 'all'
  let showFavoritesOnly = false
  let searchQuery = ''
  let editingNote: ModelNote | null = null
  let editingTags = ''
  let editingNotes = ''

  const getModelKey = (platform: string, modelName: string): string => {
    return `${platform}:${modelName}`
  }

  const getNote = (platform: string, modelName: string): ModelNote | null => {
    modelNotesKey
    const key = getModelKey(platform, modelName)
    return modelNotes.get(key) || null
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
      modelNotes = newNotes
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
      modelNotes.set(key, response.data.note)
      modelNotes = new Map(modelNotes)
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
      modelNotes.set(key, response.data.note)
      modelNotes = new Map(modelNotes)
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
      modelNotes.delete(key)
      modelNotes = new Map(modelNotes)
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

    return filtered
  }

  onMount(() => {
    loadModels()
    loadModelNotes()
  })
</script>

<div class="model-notes">
  <div class="header">
    <h2>Model Notes</h2>
    <div class="header-actions">
      <Button variant="info" onclick={loadModels} disabled={loading}>
        <MaterialIcon name="refresh" width="20" height="20" />
        Refresh Models
      </Button>
    </div>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <ModelFilters bind:selectedPlatform bind:showFavoritesOnly bind:searchQuery />

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

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    padding-bottom: 1rem;
    border-bottom: 2px solid var(--border-color, #f0f0f0);
  }

  .header h2 {
    margin: 0;
    color: var(--text-primary, #100f0f);
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
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
