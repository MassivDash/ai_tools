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
    onClose: () => void
    onSave: () => void
  }

  let {
    note,
    tags = $bindable(''),
    notes = $bindable(''),
    isFavorite = $bindable(false),
    onClose,
    onSave
  }: Props = $props()
</script>

<div class="modal-overlay" onclick={onClose}>
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h3>Edit Notes: {note.model_name}</h3>
      <Button variant="info" class="button-icon-only" onclick={onClose}>
        <MaterialIcon name="close" width="24" height="24" />
      </Button>
    </div>
    <div class="modal-content">
      <div class="form-group">
        <label>Tags (comma-separated):</label>
        <Input
          type="text"
          bind:value={tags}
          placeholder="e.g., coding, nlp, small"
        />
      </div>
      <div class="form-group">
        <label>Notes:</label>
        <textarea
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
    border-radius: 4px;
    background: var(--bg-primary, #fff);
    color: var(--text-primary, #100f0f);
    font-family: inherit;
    resize: vertical;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color, #ddd);
  }
</style>
