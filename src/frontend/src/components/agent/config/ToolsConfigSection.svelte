<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ToolInfo } from '../types'

  export let enabledTools: string[] = []
  export let onToggle: (tool: string) => void

  let availableTools: ToolInfo[] = []
  let loadingTools = false

  const loadAvailableTools = async () => {
    loadingTools = true
    try {
      const response = await axiosBackendInstance.get<ToolInfo[]>('agent/tools')
      availableTools = response.data
    } catch (err: any) {
      console.error('❌ Failed to load available tools:', err)
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

  // Group tools by category
  $: groupedTools = availableTools.reduce(
    (acc, tool) => {
      const category = tool.category || 'other'
      // category is an object or string? Backend returns "development", etc.
      // If it's an enum in Rust, it serializes to string "development" (snake_case).
      if (!acc[category]) {
        acc[category] = []
      }
      acc[category].push(tool)
      return acc
    },
    {} as Record<string, ToolInfo[]>
  )

  $: sortedCategories = Object.keys(groupedTools).sort()
</script>

<div class="tools-config">
  <div class="section-label">Tools</div>
  <p class="section-description">Select the tools the agent can use.</p>

  {#if loadingTools}
    <div class="loading">Loading tools...</div>
  {:else if availableTools.length === 0}
    <div class="no-tools">No tools available</div>
  {:else}
    <div class="tools-grid">
      {#each sortedCategories as category}
        <div class="tool-category">
          <h4 class="category-header">
            {category.charAt(0).toUpperCase() +
              category.slice(1).replace('_', ' ')}
          </h4>
          <div class="category-tools">
            {#each groupedTools[category] as tool (tool.id)}
              {@const toolTypeKey = tool.tool_type}
              <div class="tool-item">
                <label class="tool-checkbox">
                  <input
                    type="checkbox"
                    checked={isToolEnabled(toolTypeKey)}
                    on:change={() => onToggle(toolTypeKey)}
                  />
                  <span class="tool-name">{tool.name}</span>
                </label>
                <div class="tool-description">{tool.description}</div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tools-config {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .section-label {
    font-weight: 600;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .section-description {
    margin: -0.5rem 0 0 0;
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .loading,
  .no-tools {
    font-style: italic;
    color: var(--text-secondary);
    padding: 1rem;
  }

  .tools-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1.5rem;
    margin-top: 0.5rem;
    align-items: start;
  }

  .tool-category {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .category-header {
    margin: 0;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-color, #e0e0e0);
    padding-bottom: 0.25rem;
  }

  .category-tools {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .tool-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding-left: 0.5rem;
  }

  .tool-checkbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    font-weight: 500;
    font-size: 0.95rem;
    color: var(--text-primary);
  }

  .tool-checkbox input {
    cursor: pointer;
    width: 1.1rem;
    height: 1.1rem;
    accent-color: var(--accent-color, #2196f3);
  }

  .tool-description {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-left: 1.8rem; /* Align with text start */
    line-height: 1.4;
  }
</style>
