<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import Button from '@ui/Button.svelte'
  import EditableListItem from '@ui/EditableListItem.svelte'
  import type { TestQuestion } from '@types'

  export let questions: TestQuestion[] = []
  export let currentQuestionIndex = -1
  export let running = false

  const dispatch = createEventDispatcher<{
    save: { id?: number; content: string } // id missing = create
    update: { id: number; content: string }
    delete: { id: number }
    copy: { content: string }
  }>()

  let showForm = false
  let questionContent = ''
  let editingQuestionId: number | null = null

  export const openAddQuestionForm = () => {
    showForm = true
    editingQuestionId = null
    questionContent = ''
  }

  const handleSave = () => {
    if (!questionContent.trim()) return

    dispatch('save', { content: questionContent })
    showForm = false
    questionContent = ''
  }

  const handleCancel = () => {
    showForm = false
    questionContent = ''
    editingQuestionId = null
  }
</script>

{#if showForm}
  <div class="form-section">
    <textarea placeholder="Question content..." bind:value={questionContent}
    ></textarea>
    <div class="form-actions">
      <Button
        variant="primary"
        onclick={handleSave}
        disabled={!questionContent}
      >
        Add Question
      </Button>
      <Button variant="secondary" onclick={handleCancel}>Cancel</Button>
    </div>
  </div>
{/if}

<div class="list">
  {#each questions as q, i (q.id)}
    <EditableListItem
      title={q.content}
      active={i === currentQuestionIndex && running}
      on:save={(e) => dispatch('update', { id: q.id, content: e.detail })}
      on:delete={() => dispatch('delete', { id: q.id })}
      on:click={() => dispatch('copy', { content: q.content })}
    >
      <div style="display: flex; gap: 8px;">
        <span class="index">{i + 1}.</span>
        <span class="content-text">{q.content}</span>
      </div>
    </EditableListItem>
  {/each}
</div>

<style>
  .form-section {
    padding: 1rem;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  textarea {
    padding: 8px;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    font-family: inherit;
    resize: vertical;
    min-height: 60px;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .list {
    flex: 1;
    overflow-y: auto;
  }

  .index {
    font-weight: bold;
    color: var(--text-secondary);
    margin-right: 8px;
    min-width: 20px;
  }

  .content-text {
    flex: 1;
    font-size: 0.9rem;
    color: var(--text-primary);
    word-break: break-word;
  }
</style>
