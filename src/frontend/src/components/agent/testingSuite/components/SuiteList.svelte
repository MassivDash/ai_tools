<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import Button from '@ui/Button.svelte'
  import EditableListItem from '@ui/EditableListItem.svelte'
  import Input from '@ui/Input.svelte'
  import type { TestSuite } from '@types'

  export let suites: TestSuite[] = []
  export let selectedSuiteId: string | null = null

  const dispatch = createEventDispatcher<{
    select: { suite: TestSuite }
    save: { id: string; name: string; description: string }
    create: { name: string; description: string }
    delete: { id: string }
  }>()

  // Form State
  let showForm = false
  let editingSuiteId: string | null = null

  let suiteName = ''
  let suiteDescription = ''

  export const openNewSuiteForm = () => {
    showForm = true
    editingSuiteId = null
    suiteName = ''
    suiteDescription = ''
  }

  const handleCreate = () => {
    if (!suiteName.trim()) return
    dispatch('create', { name: suiteName, description: suiteDescription })
    showForm = false
    suiteName = ''
    suiteDescription = ''
  }

  const handleCancel = () => {
    showForm = false
    suiteName = ''
    suiteDescription = ''
  }
</script>

{#if showForm}
  <div class="form-section">
    <Input type="text" placeholder="Suite Name" bind:value={suiteName} />
    <Input
      type="text"
      placeholder="Description"
      bind:value={suiteDescription}
    />
    <div class="form-actions">
      <Button variant="primary" onclick={handleCreate} disabled={!suiteName}>
        Create Suite
      </Button>
      <Button variant="secondary" onclick={handleCancel}>Cancel</Button>
    </div>
  </div>
{/if}

<div class="list">
  {#each suites as suite (suite.id)}
    <EditableListItem
      title={suite.name}
      active={selectedSuiteId === suite.id}
      on:click={() => dispatch('select', { suite })}
      on:save={(e) =>
        dispatch('save', {
          id: suite.id,
          name: e.detail,
          description: suite.description || ''
        })}
      on:delete={() => dispatch('delete', { id: suite.id })}
    >
      <div class="info">
        <span class="name">{suite.name}</span>
        {#if suite.description}
          <span class="desc">{suite.description}</span>
        {/if}
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

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .list {
    flex: 1;
    overflow-y: auto;
  }

  .info {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .name {
    font-weight: 500;
    color: var(--text-primary);
  }

  .desc {
    font-size: 0.8rem;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
