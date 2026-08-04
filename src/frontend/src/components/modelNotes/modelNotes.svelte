<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import Button from '../ui/Button.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import ModelFilters from './ModelFilters.svelte'
  import ModelEditModal from './ModelEditModal.svelte'
  import PlatformSection from './PlatformSection.svelte'
  import PageSubHeader from '../ui/PageSubHeader.svelte'
  import { findNoteForModel } from '../llamaServer/config/modelMatcher'
  import type {
    LlamaModelInfo,
    OllamaModelInfo,
    ModelNote,
    ModelNoteRequest
  } from '@types'

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
    const exactNote = modelNotesData.get(key)
    if (exactNote) return exactNote

    // Find the corresponding model object
    let model: LlamaModelInfo | OllamaModelInfo | undefined
    if (platform === 'llama') {
      model = llamaModels.find(
        (m) =>
          m.name === modelName ||
          m.path === modelName ||
          m.hf_format === modelName ||
          m.legacy_hf_format === modelName
      )
    } else {
      model = ollamaModels.find((m) => m.name === modelName)
    }

    if (model) {
      return findNoteForModel(model, modelNotesData)
    }

    // Fallback to searching notes by modelName
    return findNoteForModel({ name: modelName }, modelNotesData)
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
      console.error('Failed to load models:', err)
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
      // Keyed only by each note's own model_name. Looking up a note for a
      // specific model goes through getNote's fallback (finds the real
      // model object, then findNoteForModel's owner-aware matching) rather
      // than a bare-filename shortcut here - a shared generic filename
      // (common across GGUF repos, e.g. "model-Q4_K_M.gguf") must not let
      // two different models' notes collide.
      const newNotes = new Map<string, ModelNote>()
      for (const note of response.data.notes) {
        const key = getModelKey(note.platform, note.model_name)
        newNotes.set(key, note)
      }
      modelNotesData = newNotes
      modelNotesKey++
    } catch (err: any) {
      console.error('Failed to load model notes:', err)
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
      model_name: currentNote?.model_name || modelName,
      model_path: currentNote?.model_path || modelPath,
      is_favorite: !isCurrentlyFavorite,
      tags: currentNote?.tags || [],
      notes: currentNote?.notes || undefined
    }

    try {
      const response = await axiosBackendInstance.post<{ note: ModelNote }>(
        'model-notes',
        noteRequest
      )
      // Key by the model_name the backend actually stored the row under
      // (it may differ from `modelName` here, e.g. an older note kept under
      // its original key) - otherwise this creates a second, stale-shadowed
      // entry instead of updating the one that was really persisted.
      const key = getModelKey(platform, response.data.note.model_name)
      modelNotesData.set(key, response.data.note)
      modelNotesData = new Map(modelNotesData)
      modelNotesKey++
      error = ''
    } catch (err: any) {
      console.error('Failed to toggle favorite:', err)
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
    // modelName is the model's exact path (for llama) or name (for ollama) -
    // check that first. Fall back to hf_format for notes saved under the
    // older hf_format-based key (e.g. an existing "default model" note).
    let note = getNote(platform, modelName)
    if (!note && platform === 'llama' && hfFormat) {
      note = getNote(platform, hfFormat)
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
      // Find the model to get its hf_format. editingNote.model_name may be
      // the model's exact path (its identity when no note exists yet), so
      // that has to be checked too, not just name/hf_format.
      const model = llamaModels.find(
        (m) =>
          m.name === editingNote.model_name ||
          m.path === editingNote.model_name ||
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

    const platform = editingNote.platform

    try {
      const response = await axiosBackendInstance.post<{ note: ModelNote }>(
        'model-notes',
        noteRequest
      )
      // Key by the model_name the backend actually stored the row under
      // (finalModelName may differ from editingNote.model_name, e.g. when
      // switching a note to "default" swaps it to hf_format) - otherwise
      // this creates a stale-shadowed duplicate instead of updating the row
      // that was really persisted.
      const key = getModelKey(platform, response.data.note.model_name)
      modelNotesData.set(key, response.data.note)
      modelNotesData = new Map(modelNotesData)
      modelNotesKey++
      editingNote = null
      editingTags = ''
      editingNotes = ''
      editingIsDefault = false
    } catch (err: any) {
      console.error('Failed to save note:', err)
      error = err.response?.data?.error || err.message || 'Failed to save note'
    }
  }

  const cancelEditing = () => {
    editingNote = null
    editingTags = ''
    editingNotes = ''
    editingIsDefault = false
  }

  // A model may have a note saved under any of several historical keys
  // (path, hf_format, bare name) - try each, but ask for confirmation only
  // once, and only surface a real error if every attempt failed for a
  // reason other than "no note exists under that particular key" (404).
  const deleteNote = async (platform: string, modelNames: string[]) => {
    if (modelNames.length === 0) return
    if (!confirm(`Delete notes for ${modelNames[0]}?`)) return

    let anySucceeded = false
    let realError: any = null

    for (const modelName of modelNames) {
      try {
        await axiosBackendInstance.delete(
          `model-notes/${platform}/${encodeURIComponent(modelName)}`
        )
        modelNotesData.delete(getModelKey(platform, modelName))
        anySucceeded = true
      } catch (err: any) {
        if (err.response?.status !== 404) {
          realError = err
        }
      }
    }

    if (anySucceeded) {
      modelNotesData = new Map(modelNotesData)
      modelNotesKey++
    }

    if (realError) {
      console.error('Failed to delete note:', realError)
      error =
        realError.response?.data?.error ||
        realError.message ||
        'Failed to delete note'
    } else if (anySucceeded) {
      error = ''
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

<div class="model-notes">
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
      {isDefault}
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
    max-width: calc(100% - 5rem);
    margin: 0 auto;
  }

  @media screen and (max-width: 768px) {
    .model-notes {
      /* Match Agent Chat's mobile spacing between the config buttons row
         and the content below it. */
      margin-top: 1rem;
    }
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
