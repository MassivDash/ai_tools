<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type {
    ToolGroup,
    ToolGroupResponse,
    ToolGroupsResponse,
    ToolInfo
  } from '@types'
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  import CheckboxWithHelp from '@ui/CheckboxWithHelp.svelte'
  import Input from '@ui/Input.svelte'
  import Button from '@ui/Button.svelte'
  import Modal from '@ui/Modal.svelte'
  import { deriveToolEntries, formatLabel, type ToolEntry } from './toolEntries'

  export let enabledTools: string[] = []
  export let onApply: (_toolTypes: string[]) => void
  export let onRemove: (_toolTypes: string[]) => void

  let groups: ToolGroup[] = []
  let toolEntries: ToolEntry[] = []
  let loadingGroups = false
  let error = ''

  let modalOpen = false
  let editingId: number | null = null
  let groupName = ''
  let selectedTypes: string[] = []

  let toastMessage = ''
  let toastTimeout: ReturnType<typeof setTimeout> | undefined

  const showToast = (message: string) => {
    clearTimeout(toastTimeout)
    toastMessage = message
    toastTimeout = setTimeout(() => {
      toastMessage = ''
    }, 2500)
  }

  $: isGroupApplied = (group: ToolGroup) =>
    group.tool_types.every((toolType) => enabledTools.includes(toolType))

  const applyGroup = (group: ToolGroup) => {
    onApply(group.tool_types)
    showToast(`Applied "${group.name}"`)
  }

  const removeGroup = (group: ToolGroup) => {
    onRemove(group.tool_types)
    showToast(`Removed "${group.name}"`)
  }

  const loadGroups = async () => {
    try {
      const response =
        await axiosBackendInstance.get<ToolGroupsResponse>('agent/tool-groups')
      groups = response.data.groups
    } catch (err: any) {
      console.error('Failed to load tool groups:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to load tool groups'
    }
  }

  const loadTools = async () => {
    try {
      const response = await axiosBackendInstance.get<ToolInfo[]>('agent/tools')
      toolEntries = deriveToolEntries(response.data)
    } catch (err: any) {
      console.error('Failed to load tools:', err)
    }
  }

  onMount(() => {
    loadingGroups = true
    Promise.all([loadGroups(), loadTools()]).finally(() => {
      loadingGroups = false
    })
  })

  const openCreateModal = () => {
    editingId = null
    groupName = ''
    selectedTypes = []
    error = ''
    modalOpen = true
  }

  const openEditModal = (group: ToolGroup) => {
    editingId = group.id
    groupName = group.name
    selectedTypes = [...group.tool_types]
    error = ''
    modalOpen = true
  }

  const closeModal = () => {
    modalOpen = false
  }

  const isSelected = (toolType: string) => selectedTypes.includes(toolType)

  const toggleSelected = (toolType: string) => {
    selectedTypes = isSelected(toolType)
      ? selectedTypes.filter((t) => t !== toolType)
      : [...selectedTypes, toolType]
  }

  const saveGroup = async () => {
    const name = groupName.trim()
    if (!name || selectedTypes.length === 0) return

    try {
      if (editingId === null) {
        const response = await axiosBackendInstance.post<ToolGroupResponse>(
          'agent/tool-groups',
          { name, tool_types: selectedTypes }
        )
        groups = [...groups, response.data.group]
      } else {
        const response = await axiosBackendInstance.put<ToolGroupResponse>(
          `agent/tool-groups/${editingId}`,
          { name, tool_types: selectedTypes }
        )
        groups = groups.map((g) =>
          g.id === editingId ? response.data.group : g
        )
      }
      closeModal()
    } catch (err: any) {
      console.error('Failed to save tool group:', err)
      error =
        err.response?.data?.error || err.message || 'Failed to save tool group'
    }
  }

  const deleteGroup = async (group: ToolGroup) => {
    if (!confirm(`Delete group "${group.name}"?`)) return

    try {
      await axiosBackendInstance.delete(`agent/tool-groups/${group.id}`)
      groups = groups.filter((g) => g.id !== group.id)
    } catch (err: any) {
      console.error('Failed to delete tool group:', err)
      error =
        err.response?.data?.error ||
        err.message ||
        'Failed to delete tool group'
    }
  }
</script>

<div class="tool-groups-config">
  <div class="section-label">Tool Groups</div>
  <p class="section-description">
    Save named bundles of tools so you can activate several at once.
  </p>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if toastMessage}
    <div class="toast" role="status">{toastMessage}</div>
  {/if}

  <div class="groups-toolbar">
    <Button variant="primary" size="small" onclick={openCreateModal}>
      <MaterialIcon name="plus" width="16" height="16" />
      New Group
    </Button>
  </div>

  {#if loadingGroups}
    <div class="loading">Loading tool groups...</div>
  {:else if groups.length === 0}
    <div class="no-groups">No tool groups yet</div>
  {:else}
    <div class="groups-list">
      {#each groups as group (group.id)}
        <div class="group-row">
          <div class="group-main">
            <div class="group-name">{group.name}</div>
            <div class="group-tags">
              {#each group.tool_types as toolType (toolType)}
                <span class="tool-tag">{formatLabel(toolType)}</span>
              {/each}
            </div>
          </div>
          <div class="group-actions">
            {#if isGroupApplied(group)}
              <Button
                variant="danger"
                size="small"
                onclick={() => removeGroup(group)}
              >
                Remove
              </Button>
            {:else}
              <Button
                variant="primary"
                size="small"
                onclick={() => applyGroup(group)}
              >
                Apply
              </Button>
            {/if}
            <Button
              variant="secondary"
              size="small"
              onclick={() => openEditModal(group)}
            >
              Edit
            </Button>
            <Button
              variant="danger"
              size="small"
              onclick={() => deleteGroup(group)}
            >
              Delete
            </Button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<Modal
  isOpen={modalOpen}
  title={editingId === null ? 'New Tool Group' : 'Edit Tool Group'}
  on:close={closeModal}
>
  <Input type="text" placeholder="Group name" bind:value={groupName} />
  <div class="tool-picker">
    {#each toolEntries as entry (entry.toolType)}
      <CheckboxWithHelp
        checked={isSelected(entry.toolType)}
        onchange={() => toggleSelected(entry.toolType)}
        label={entry.displayName}
        helpText={entry.description}
      />
    {/each}
  </div>
  <svelte:fragment slot="footer">
    <Button variant="secondary" onclick={closeModal}>Cancel</Button>
    <Button
      variant="primary"
      onclick={saveGroup}
      disabled={!groupName.trim() || selectedTypes.length === 0}
    >
      Save
    </Button>
  </svelte:fragment>
</Modal>

<style>
  .tool-groups-config {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    margin-bottom: 2rem;
  }

  .section-label {
    font-weight: 600;
    color: var(--text-primary);
    font-size: 1.1rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .section-description {
    margin: -1rem 0 0 0;
    font-size: 0.95rem;
    color: var(--text-secondary);
  }

  .groups-toolbar {
    display: flex;
    justify-content: flex-end;
  }

  .loading,
  .no-groups {
    font-style: italic;
    color: var(--text-secondary);
    padding: 2rem;
    text-align: center;
    background: var(--bg-secondary, #f5f5f5);
    border-radius: 8px;
  }

  .groups-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .group-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    background: var(--bg-primary, #ffffff);
  }

  .group-main {
    min-width: 0;
    flex: 1;
  }

  .group-name {
    font-weight: 600;
    color: var(--text-primary);
  }

  .group-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-top: 0.35rem;
  }

  .tool-tag {
    padding: 0.15rem 0.5rem;
    background: var(--bg-secondary, #f0f0f0);
    border-radius: 8px;
    font-size: 0.75rem;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .group-actions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .tool-picker {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 20rem;
    overflow-y: auto;
    margin-top: 1rem;
  }

  .error {
    padding: 0.75rem;
    background-color: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 8px;
    color: var(--accent-color, #c33);
    font-size: 0.9rem;
  }

  .toast {
    padding: 0.6rem 1rem;
    background-color: var(--md-tertiary-container, #d7f0d7);
    color: var(--md-on-tertiary-container, #1e3a1e);
    border-radius: 8px;
    font-size: 0.9rem;
    font-weight: 600;
  }
</style>
