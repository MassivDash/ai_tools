<script lang="ts">
  import { onMount } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { ToolInfo } from '@types'
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  import CheckboxWithHelp from '@ui/CheckboxWithHelp.svelte'
  export let enabledTools: string[] = []
  export let onToggle: (_tool: string) => void

  let availableTools: ToolInfo[] = []
  let loadingTools = false

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

  // Group tools by category and then by tool_type
  $: categoryGroups = availableTools.reduce(
    (acc, tool) => {
      const category = tool.category || 'other'
      const type = tool.tool_type

      // Skip ChromaDB as it has its own configuration section
      if (type === 'chroma_d_b') {
        return acc
      }

      if (!acc[category]) {
        acc[category] = {}
      }

      if (!acc[category][type]) {
        acc[category][type] = []
      }
      acc[category][type].push(tool)

      return acc
    },
    {} as Record<string, Record<string, ToolInfo[]>>
  )

  $: sortedCategories = Object.keys(categoryGroups).sort()
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
        <!-- Get icon from the first tool in this category -->
        {@const firstToolType = Object.keys(categoryGroups[category])[0]}
        {@const categoryIcon = categoryGroups[category][firstToolType][0].icon}

        <div class="tool-category">
          <h4 class="category-header">
            <MaterialIcon
              name={categoryIcon}
              width="20"
              height="20"
              class="category-icon"
            />
            <span>
              {category.charAt(0).toUpperCase() +
                category.slice(1).replace('_', ' ')}
            </span>
          </h4>
          <div class="category-tools">
            {#each Object.entries(categoryGroups[category]) as [toolType, tools] (toolType)}
              {@const isEnabled = isToolEnabled(toolType)}
              <!-- Determine display name and description -->
              {@const displayName =
                tools.length > 1
                  ? toolType.charAt(0).toUpperCase() +
                    toolType.slice(1).replace(/_/g, ' ')
                  : tools[0].name}
              {@const description =
                tools.length > 1
                  ? tools.map((t) => t.description).join('. ')
                  : tools[0].description}

              <div class="tool-item">
                <CheckboxWithHelp
                  checked={isEnabled}
                  onchange={() => onToggle(toolType)}
                  label={displayName}
                  helpText={description}
                />
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

  .tools-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1.5rem;
    margin-top: 1rem;
    align-items: start;
  }

  .tool-category {
    display: flex;
    flex-direction: column;
    gap: 0;
    background: var(--bg-primary, #ffffff);
    border-radius: 8px;
    border: 1px solid var(--border-color, #e0e0e0);
    overflow: hidden;
    transition:
      box-shadow 0.2s ease,
      transform 0.2s ease;
  }

  .tool-category:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
    border-color: var(--accent-color-alpha, rgba(33, 150, 243, 0.3));
  }

  .category-header {
    margin: 0;
    padding: 0.75rem 1rem;
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-primary);
    font-weight: 600;
    background-color: var(--bg-secondary, #f8f9fa);
    border-bottom: 1px solid var(--border-color, #e0e0e0);
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  /* Target the icon specifically if needed, but flex handles alignment */

  .category-tools {
    display: flex;
    flex-direction: column;
  }

  .tool-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.75rem 1rem;
    transition: background-color 0.15s ease;
    border-bottom: 1px solid transparent;
  }

  .tool-item:not(:last-child) {
    border-bottom: 1px solid var(--border-color-light, #f0f0f0);
  }

  .tool-item:hover {
    background-color: var(--bg-tertiary, #fafafa);
  }
</style>
