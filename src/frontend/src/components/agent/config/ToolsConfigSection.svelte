<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ToolInfo } from '@types'
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  import CheckboxWithHelp from '@ui/CheckboxWithHelp.svelte'
  import Input from '@ui/Input.svelte'
  import Button from '@ui/Button.svelte'
  export let enabledTools: string[] = []
  export let onToggle: (_tool: string) => void

  let availableTools: ToolInfo[] = []
  let loadingTools = false
  let searchQuery = ''
  let groupByCategory = false

  const loadAvailableTools = async () => {
    loadingTools = true
    try {
      const response = await axiosBackendInstance.get<ToolInfo[]>('agent/tools')
      availableTools = response.data
    } catch (err: any) {
      console.error('Failed to load available tools:', err)
      availableTools = []
    } finally {
      loadingTools = false
    }
  }

  onMount(() => {
    loadAvailableTools()
  })

  $: isToolEnabled = (toolType: string) => {
    return enabledTools.includes(toolType)
  }

  const formatLabel = (value: string) =>
    value.charAt(0).toUpperCase() + value.slice(1).replace(/_/g, ' ')

  // Collapse tools sharing a tool_type into a single row (e.g. Gmail read/write)
  $: toolEntries = (() => {
    const byType = new Map<string, ToolInfo[]>()
    for (const tool of availableTools) {
      const list = byType.get(tool.tool_type) || []
      list.push(tool)
      byType.set(tool.tool_type, list)
    }
    return Array.from(byType.entries()).map(([toolType, tools]) => ({
      toolType,
      category: tools[0].category || 'other',
      icon: tools[0].icon,
      displayName: tools.length > 1 ? formatLabel(toolType) : tools[0].name,
      description:
        tools.length > 1
          ? tools.map((t) => t.description).join('. ')
          : tools[0].description
    }))
  })()

  $: filteredEntries = toolEntries.filter((entry) => {
    if (!searchQuery.trim()) return true
    const query = searchQuery.toLowerCase()
    return (
      entry.displayName.toLowerCase().includes(query) ||
      entry.description.toLowerCase().includes(query) ||
      entry.category.toLowerCase().includes(query)
    )
  })

  $: sortedEntries = groupByCategory
    ? [...filteredEntries].sort(
        (a, b) =>
          a.category.localeCompare(b.category) ||
          a.displayName.localeCompare(b.displayName)
      )
    : [...filteredEntries].sort((a, b) =>
        a.displayName.localeCompare(b.displayName)
      )
</script>

<div class="tools-config">
  <div class="section-label">Tools</div>
  <p class="section-description">Select the tools the agent can use.</p>

  {#if loadingTools}
    <div class="loading">Loading tools...</div>
  {:else if availableTools.length === 0}
    <div class="no-tools">No tools available</div>
  {:else}
    <div class="tools-toolbar">
      <div class="search-wrapper">
        <Input
          type="text"
          placeholder="Search tools..."
          bind:value={searchQuery}
        />
      </div>
      <Button
        variant={groupByCategory ? 'primary' : 'secondary'}
        size="small"
        onclick={() => (groupByCategory = !groupByCategory)}
        title="Group tools by category"
      >
        <MaterialIcon name="view-agenda" width="16" height="16" />
        Grouped
      </Button>
    </div>

    <div class="tools-list">
      {#if sortedEntries.length === 0}
        <div class="no-results">No tools match "{searchQuery}"</div>
      {:else}
        {#each sortedEntries as entry, i (entry.toolType)}
          {@const showDivider =
            groupByCategory &&
            (i === 0 || sortedEntries[i - 1].category !== entry.category)}
          {#if showDivider}
            <div class="category-divider">
              <span>{formatLabel(entry.category)}</span>
            </div>
          {/if}
          <div class="tool-row">
            <MaterialIcon name={entry.icon} width="20" height="20" class="tool-icon" />
            <div class="tool-main">
              <CheckboxWithHelp
                checked={isToolEnabled(entry.toolType)}
                onchange={() => onToggle(entry.toolType)}
                label={entry.displayName}
                helpText={entry.description}
              />
            </div>
            {#if !groupByCategory}
              <span class="category-tag">{formatLabel(entry.category)}</span>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .tools-config {
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

  .loading,
  .no-tools {
    font-style: italic;
    color: var(--text-secondary);
    padding: 2rem;
    text-align: center;
    background: var(--bg-secondary, #f5f5f5);
    border-radius: 8px;
  }

  .tools-toolbar {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .search-wrapper {
    flex: 1;
  }

  .search-wrapper :global(.input-wrapper) {
    margin-bottom: 0;
  }

  .tools-toolbar :global(.button) {
    margin-top: 0.05rem;
  }

  .tools-list {
    display: flex;
    flex-direction: column;
    max-height: 32rem;
    overflow-y: auto;
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    background: var(--bg-primary, #ffffff);
  }

  .no-results {
    font-style: italic;
    color: var(--text-secondary);
    padding: 2rem;
    text-align: center;
  }

  .category-divider {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 0.5rem 1rem;
    font-size: 0.8rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    background-color: var(--bg-secondary, #f8f9fa);
    border-bottom: 1px solid var(--border-color, #e0e0e0);
  }

  .tool-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    transition: background-color 0.15s ease;
  }

  .tool-row:not(:last-child) {
    border-bottom: 1px solid var(--border-color-light, #f0f0f0);
  }

  .tool-row:hover {
    background-color: var(--bg-tertiary, #fafafa);
  }

  .tool-row :global(.tool-icon) {
    flex-shrink: 0;
    margin-top: 0.2rem;
    color: var(--text-secondary);
  }

  .tool-main {
    flex: 1;
    min-width: 0;
  }

  .tool-main :global(.checkbox-with-help) {
    margin-bottom: 0;
  }

  .category-tag {
    flex-shrink: 0;
    padding: 0.2rem 0.5rem;
    background: var(--bg-secondary, #f0f0f0);
    border-radius: 8px;
    font-size: 0.75rem;
    color: var(--text-secondary);
    white-space: nowrap;
  }
</style>
