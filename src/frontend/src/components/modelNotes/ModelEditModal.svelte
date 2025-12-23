<script lang="ts">
  import Button from '../ui/Button.svelte'
  import Input from '../ui/Input.svelte'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import type { ModelNote } from './types'

  interface Props {
    note: ModelNote
    tags?: string
    notes?: string
    isFavorite?: boolean
    isDefault?: boolean
    onClose: () => void
    onSave: () => void
  }

  let {
    note,
    tags = $bindable(''),
    notes = $bindable(''),
    isFavorite = $bindable(false),
    isDefault = $bindable(false),
    onClose,
    onSave
  }: Props = $props()

  const handleOverlayClick = (e: MouseEvent) => {
    // Click outside modal to close
    if (e.target === e.currentTarget) {
      onClose()
    }
  }

  const handleOverlayKeydown = (e: KeyboardEvent) => {
    // Handle Escape key to close modal
    if (e.key === 'Escape') {
      e.preventDefault()
      onClose()
    }
  }

  const tagsInputId = $derived(`tags-${note.id || Math.random()}`)
  const notesTextareaId = $derived(`notes-${note.id || Math.random()}`)
</script>

<div
  class="modal-overlay"
  role="dialog"
  aria-modal="true"
  aria-labelledby="modal-title"
  tabindex="-1"
  onclick={handleOverlayClick}
  onkeydown={handleOverlayKeydown}
>
  <div class="modal" role="document">
    <div class="modal-header">
      <h3 id="modal-title">Edit Notes: {note.model_name}</h3>
      <Button variant="info" class="button-icon-only" onclick={onClose}>
        <MaterialIcon name="close" width="24" height="24" />
      </Button>
    </div>
    <div class="modal-content">
      <div class="form-group">
        <label for={tagsInputId}>Tags (comma-separated):</label>
        <Input
          id={tagsInputId}
          type="text"
          bind:value={tags}
          placeholder="e.g., coding, nlp, small"
        />
      </div>
      <div class="form-group">
        <label for={notesTextareaId}>Notes:</label>
        <textarea
          id={notesTextareaId}
          bind:value={notes}
          placeholder="Add your notes about this model..."
          rows="6"
        ></textarea>
      </div>
      <div class="form-group">
        <label>
          <input type="checkbox" bind:checked={isFavorite} />
          Favorite
        </label>
      </div>
      <div class="form-group">
        <label>
          <input type="checkbox" bind:checked={isDefault} />
          Set as Default {note.platform === 'llama' ? 'Llama' : 'Ollama'} Model
        </label>
        <p class="help-text">
          {#if note.platform === 'llama'}
            This will be used as the default model for the Llama server. Only
            one model can be default.
          {:else}
            This will be used as the default model for ChromaDB and Agent
            (Ollama). Only one model can be default.
          {/if}
        </p>
      </div>
    </div>
    <div class="modal-footer">
      <Button variant="info" onclick={onClose}>Cancel</Button>
      <Button variant="success" onclick={onSave}>Save</Button>
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-primary, #fff);
    border-radius: 8px;
    padding: 1.5rem;
    max-width: 600px;
    width: 90%;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 4px 16px var(--shadow, rgba(0, 0, 0, 0.2));
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-color, #ddd);
  }

  .modal-header h3 {
    margin: 0;
    color: var(--text-primary, #100f0f);
  }

  .modal-content {
    margin-bottom: 1rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
    color: var(--text-primary, #100f0f);
  }

  .form-group textarea {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--border-color, #ddd);
    border-radius: 8px;
    background: var(--bg-primary, #fff);
    color: var(--text-primary, #100f0f);
    font-family: inherit;
    resize: vertical;
  }

  .help-text {
    margin-top: 0.25rem;
    font-size: 0.85rem;
    color: var(--text-secondary, #666);
    font-style: italic;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color, #ddd);
  }
</style>
