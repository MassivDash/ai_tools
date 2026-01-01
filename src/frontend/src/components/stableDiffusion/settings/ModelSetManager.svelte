<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte'
  import { stableDiffusionApi } from '../../../api/stableDiffusion'
  import Button from '../../ui/Button.svelte'
  import Input from '../../ui/Input.svelte'
  import CheckboxWithHelp from '../../ui/CheckboxWithHelp.svelte'
  import IconButton from '../../ui/IconButton.svelte'
  import MaterialIcon from '../../ui/MaterialIcon.svelte'
  import EditableListItem from '../../ui/EditableListItem.svelte'
  import { type SDModelSet } from '../../../validation/stableDiffusion'

  export let onSelectSet: ((set: SDModelSet) => void) | undefined = undefined

  let modelSets: SDModelSet[] = []
  let loading = false
  let error = ''
  let message = ''

  // Form State
  let showForm = false
  let editingId: number | null = null
  let formName = ''
  let formDiffusion = ''
  let formVae = ''
  let formLlm = ''
  let formIsDefault = false

  const fetchSets = async () => {
    loading = true
    try {
      modelSets = await stableDiffusionApi.getModelSets()
    } catch (e) {
      error = 'Failed to load model sets'
    } finally {
      loading = false
    }
  }

  const deleteSet = async (id: number) => {
    if (!confirm('Are you sure you want to delete this set?')) return
    try {
      await stableDiffusionApi.deleteModelSet(id)
      await fetchSets()
    } catch (e) {
      error = 'Failed to delete set'
    }
  }

  const editSet = (set: SDModelSet) => {
    editingId = set.id
    formName = set.name
    formDiffusion = set.diffusion_model
    formVae = set.vae || ''
    formLlm = set.llm || ''
    formIsDefault = set.is_default
    showForm = true
    error = ''
  }

  const startNewSet = () => {
    editingId = null
    formName = ''
    formDiffusion = ''
    formVae = ''
    formLlm = ''
    formIsDefault = false
    showForm = true
    error = ''
  }

  const saveSet = async () => {
    if (!formName || !formDiffusion) {
      error = 'Name and Diffusion Model are required'
      return
    }

    const payload = {
      name: formName,
      diffusion_model: formDiffusion,
      vae: formVae || null,
      llm: formLlm || null,
      is_default: formIsDefault
    }

    try {
      if (editingId) {
        await stableDiffusionApi.updateModelSet(editingId, payload)
      } else {
        await stableDiffusionApi.createModelSet(payload)
      }
      showForm = false
      await fetchSets()
      message = editingId ? 'Set updated' : 'Set created'
      setTimeout(() => (message = ''), 3000)
    } catch (e: any) {
      error = e.message || 'Failed to save set'
    }
  }

  const renameSet = async (id: number, newName: string) => {
    try {
      await stableDiffusionApi.updateModelSet(id, { name: newName })
      await fetchSets()
    } catch (e: any) {
      error = e.message || 'Failed to rename set'
    }
  }

  const selectSet = (set: SDModelSet) => {
    if (onSelectSet) {
      onSelectSet(set)
    }
  }

  onMount(() => {
    fetchSets()
  })
</script>

<div class="model-sets-manager">
  <div class="header">
    <h4>Model Sets</h4>
    <Button variant="secondary" size="small" onclick={startNewSet}>
      <MaterialIcon name="plus" width="18" height="18" />
      New Set
    </Button>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}
  {#if message}
    <div class="success">{message}</div>
  {/if}

  {#if showForm}
    <div class="form-panel">
      <h5>{editingId ? 'Edit Model Set' : 'Create Model Set'}</h5>
      <div class="form-row">
        <Input
          label="Set Name"
          bind:value={formName}
          placeholder="e.g. Turbo"
        />
      </div>
      <div class="form-row">
        <Input
          label="Diffusion Model Filename"
          bind:value={formDiffusion}
          placeholder="model.gguf"
        />
      </div>
      <div class="form-row">
        <Input
          label="VAE Filename (Optional)"
          bind:value={formVae}
          placeholder="vae.safetensors"
        />
      </div>
      <div class="form-row">
        <Input
          label="LLM / CLIP Filename (Optional)"
          bind:value={formLlm}
          placeholder="clip_encoder.gguf"
        />
      </div>
      <div class="form-row">
        <CheckboxWithHelp
          label="Set as Default"
          helpText="Load this set automatically on server start"
          bind:checked={formIsDefault}
        />
      </div>
      <div class="action-row">
        <Button
          variant="secondary"
          size="small"
          onclick={() => (showForm = false)}
        >
          Cancel
        </Button>
        <Button variant="primary" size="small" onclick={saveSet}>Save</Button>
      </div>
    </div>
  {:else}
    <div class="sets-list">
      {#if loading}
        <div class="loading">
          <MaterialIcon name="loading" width="24" height="24" />
          Loading sets...
        </div>
      {:else if modelSets.length === 0}
        <p class="empty">No sets defined.</p>
      {:else}
        {#each modelSets as set (set.id)}
          <!-- Reusing EditableListItem for consistent look -->
          <EditableListItem
            title={set.name}
            model={set.diffusion_model}
            active={false}
            allowEdit={true}
            allowDelete={true}
            on:click={() => selectSet(set)}
            on:save={(e) => renameSet(set.id, e.detail)}
            on:delete={() => deleteSet(set.id)}
          >
            <!-- Custom content slot if needed, but we use title prop -->
            <!-- Inject extra actions -->
            <div slot="actions-start" style="display:contents">
              <button
                class="action-btn"
                on:click|stopPropagation={() => editSet(set)}
                title="Edit Details"
              >
                <MaterialIcon name="tune" width="18" height="18" />
              </button>
            </div>
          </EditableListItem>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .model-sets-manager {
    /* Clean look */
  }
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }
  h4 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }
  h5 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
  }
  .sets-list {
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border-radius: 6px;
    overflow: hidden; /* For border radius with list items */
    border: 1px solid var(--border-color);
  }

  .form-panel {
    background: var(--bg-surface);
    padding: 1rem;
    border-radius: 6px;
    border: 1px solid var(--border-color);
  }
  .form-row {
    margin-bottom: 1rem;
  }
  .action-row {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .error {
    color: var(--error-color);
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }
  .success {
    color: var(--success-color);
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }
  .empty {
    color: var(--text-secondary);
    font-style: italic;
    text-align: center;
    padding: 1rem;
  }
  .loading {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    justify-content: center;
    padding: 1rem;
  }

  /* Style the injected action button to match EditableListItem's internal buttons */
  .action-btn {
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
  }

  .action-btn:hover {
    color: var(--md-primary);
    background-color: var(--md-surface-variant);
    border-radius: 50%;
  }
</style>
