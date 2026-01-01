<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import MaterialIcon from './MaterialIcon.svelte'

  export let title: string = ''
  export let model: string | undefined = undefined
  export let active: boolean = false
  export let allowEdit: boolean = true
  export let allowDelete: boolean = true

  const dispatch = createEventDispatcher<{
    click: void
    save: string
    delete: void
  }>()

  let isEditing = false
  let editValue = ''
  let isDeleting = false

  function startEdit(e: Event) {
    e.stopPropagation()
    isEditing = true
    editValue = title
  }

  function save() {
    if (editValue.trim() !== title) {
      dispatch('save', editValue)
    }
    isEditing = false
  }

  function startDelete(e: Event) {
    e.stopPropagation()
    isDeleting = true
  }

  function confirmDelete(e: Event) {
    e.stopPropagation()
    dispatch('delete')
    isDeleting = false
  }

  function cancelDelete(e: Event) {
    e.stopPropagation()
    isDeleting = false
  }

  function handleKeypress(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      dispatch('click')
    }
  }

  function handleEditKeypress(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      save()
    }
  }
</script>

<div
  class="item"
  class:active
  on:click={() => !isEditing && !isDeleting && dispatch('click')}
  on:keypress={handleKeypress}
  role="button"
  tabindex="0"
>
  {#if isEditing}
    <input
      type="text"
      bind:value={editValue}
      on:click|stopPropagation
      on:keypress|stopPropagation={handleEditKeypress}
      on:blur={save}
      autoFocus
    />
  {:else if isDeleting}
    <div class="confirm-delete">
      <span>Delete?</span>
      <button class="confirm-btn" on:click={confirmDelete}> Yes </button>
      <button class="cancel-btn" on:click={cancelDelete}> No </button>
    </div>
  {:else}
    <div class="content" title={title + (model ? ` (${model})` : '')}>
      <slot>
        <span class="title">
          <span class="title-text">{title}</span>
          {#if model}
            <span class="model-badge">{model}</span>
          {/if}
        </span>
      </slot>
    </div>
    <div class="item-actions">
      <slot name="actions-start" />
      {#if allowEdit}
        <button class="action-btn" on:click={startEdit} title="Rename">
          <MaterialIcon name="pencil" width="18" height="18" />
        </button>
      {/if}
      {#if allowDelete}
        <button class="action-btn delete" on:click={startDelete} title="Delete">
          <MaterialIcon name="delete" width="18" height="18" />
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .item {
    padding: 1rem;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-color);
    transition: background 0.2s;
    min-height: 4rem;
    background-color: var(--md-surface);
  }

  .item:hover {
    background-color: var(--md-secondary-container);
    color: var(--md-on-secondary-container);
  }

  .item.active {
    background-color: var(--md-accent);
    border-left: 3px solid var(--md-primary);
  }

  .content {
    flex: 1;
    overflow: hidden;
    margin-right: 8px;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .title {
    display: flex;
    flex-direction: column; /* Stack vertically */
    align-items: flex-start; /* Align left */
    gap: 2px;
    width: 100%;
  }

  .title-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 1rem; /* Slightly larger title */
    font-weight: 500;
    color: var(--text-primary);
    width: 100%;
  }

  .model-badge {
    font-size: 0.75rem;
    color: var(--text-secondary);
    background: transparent; /* Remove background for cleaner look in column */
    padding: 0;
    margin-top: 2px;
    border-radius: 0;
    flex-shrink: 0;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-actions {
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.2s;
  }

  .item:hover .item-actions {
    opacity: 1;
  }

  .action-btn {
    background: none;
    border: none;
    padding: 4px; /* Larger hit area */
    cursor: pointer;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
  }

  .action-btn:hover {
    color: var(--md-primary);
    background-color: var(--md-surface-variant); /* Hover validation */
    border-radius: 50%;
  }

  .action-btn.delete:hover {
    color: var(--md-error);
    background-color: rgba(183, 28, 28, 0.1);
  }

  .confirm-delete {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    font-size: 0.9rem;
  }

  .confirm-btn {
    background: var(--md-error);
    color: var(--md-on-error);
    border: none;
    border-radius: 8px;
    padding: 4px 12px;
    cursor: pointer;
  }

  .cancel-btn {
    background: var(--md-surface-variant);
    color: var(--md-on-surface-variant);
    border: none;
    border-radius: 8px;
    padding: 4px 12px;
    cursor: pointer;
  }

  input[type='text'] {
    width: 100%;
    padding: 8px; /* Larger padding for input */
    border: 1px solid var(--md-primary);
    border-radius: 8px;
    outline: none;
    font-family: inherit;
    font-size: 1rem;
  }
</style>
