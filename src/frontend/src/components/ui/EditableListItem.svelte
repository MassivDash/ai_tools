<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import MaterialIcon from './MaterialIcon.svelte'

  export let title: string = ''
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
    <div class="content">
      <slot>
        <span class="title" {title}>{title}</span>
      </slot>
    </div>
    <div class="item-actions">
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
    padding: 0.75rem 1rem;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-color);
    transition: background 0.2s;
    min-height: 3rem;
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
  }

  .title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.9rem;
    color: var(--text-primary);
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
    padding: 2px;
    cursor: pointer;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
  }

  .action-btn:hover {
    color: var(--md-primary);
  }

  .action-btn.delete:hover {
    color: var(--md-error);
  }

  .confirm-delete {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    font-size: 0.85rem;
  }

  .confirm-btn {
    background: var(--md-error);
    color: var(--md-on-error);
    border: none;
    border-radius: 8px;
    padding: 2px 8px;
    cursor: pointer;
  }

  .cancel-btn {
    background: var(--md-surface-variant);
    color: var(--md-on-surface-variant);
    border: none;
    border-radius: 8px;
    padding: 2px 8px;
    cursor: pointer;
  }

  input[type='text'] {
    width: 100%;
    padding: 4px;
    border: 1px solid var(--md-primary);
    border-radius: 8px;
    outline: none;
    font-family: inherit;
    font-size: 0.9rem;
  }
</style>
