<script lang="ts">
  import { onMount } from 'svelte'
  import HelpIcon from '../../ui/HelpIcon.svelte'
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
      // Fallback to empty array
      availableTools = []
    } finally {
      loadingTools = false
    }
  }

  onMount(() => {
    loadAvailableTools()
  })

  $: toolEnabled = (toolType: string) => {
    // tool_type from backend matches enabled_tools format (snake_case)
    return enabledTools.includes(toolType)
  }
</script>

<div class="config-section">
  <div class="section-label">Tools:</div>
  {#if loadingTools}
    <div class="loading">Loading tools...</div>
  {:else if availableTools.length === 0}
    <div class="no-tools">No tools available</div>
  {:else}
    <div class="tools-list">
      {#each availableTools as tool (tool.id)}
        {@const toolTypeKey = tool.tool_type}
        <label class="tool-checkbox">
          <input
            type="checkbox"
            checked={toolEnabled(toolTypeKey)}
            onchange={() => onToggle(toolTypeKey)}
            class="checkbox-input"
          />
          <span>{tool.name}</span>
          <HelpIcon text={tool.description} />
        </label>
      {/each}
    </div>
  {/if}
</div>

<style>
  .config-section {
    margin-bottom: 2rem;
  }

  .section-label {
    display: block;
    margin-bottom: 0.75rem;
    font-weight: 600;
    color: var(--text-primary, #333);
    font-size: 1rem;
    transition: color 0.3s ease;
  }

  .tools-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .tool-checkbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    font-weight: 600;
    color: var(--text-primary, #333);
    transition: color 0.3s ease;
  }

  .checkbox-input {
    width: 1.25rem;
    height: 1.25rem;
    cursor: pointer;
    accent-color: var(--accent-color, #2196f3);
  }

  .loading,
  .no-tools {
    padding: 0.75rem;
    color: var(--text-secondary, #666);
    font-size: 0.9rem;
    transition: color 0.3s ease;
  }
</style>
