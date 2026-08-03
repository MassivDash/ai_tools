<script lang="ts">
  import Button from '@ui/Button.svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
  import type { AgentConfig, AgentConfigResponse } from '@types'
  import ToolsConfigSection from './ToolsConfigSection.svelte'
  import ToolGroupsConfigSection from './ToolGroupsConfigSection.svelte'
  import CheckboxWithHelp from '@ui/CheckboxWithHelp.svelte'
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  export let isOpen: boolean = false
  export let onClose: () => void
  export let onSave: () => void

  let activeTab: 'tools' | 'groups' = 'tools'
  let enabledTools: string[] = []
  let debugLogging = false
  let savingConfig = false
  let error = ''

  const loadConfig = async () => {
    try {
      const response =
        await axiosBackendInstance.get<AgentConfig>('agent/config')
      // Backend returns enabled_tools as string[] (ToolType enum serialized to snake_case)
      enabledTools = response.data.enabled_tools || []
      debugLogging = !!response.data.debug_logging
    } catch (err: any) {
      console.error('Failed to load agent config:', err)
    }
  }

  $: if (isOpen) {
    loadConfig().catch(console.error)
  }

  const handleToolToggle = (tool: string) => {
    if (enabledTools.includes(tool)) {
      enabledTools = enabledTools.filter((t) => t !== tool)
    } else {
      enabledTools = [...enabledTools, tool]
    }
  }

  // Persists the current enabledTools/debugLogging immediately. Used both by the
  // footer's Save button and by group Apply/Remove/Clear All, which must take real
  // effect right away rather than only being staged until Save is clicked.
  const persistConfig = async (): Promise<boolean> => {
    error = ''

    try {
      const payload = {
        enabled_tools: enabledTools, // Already in correct format from tool.tool_type
        debug_logging: debugLogging
      }

      const response = await axiosBackendInstance.post<AgentConfigResponse>(
        'agent/config',
        payload
      )

      if (response.data.success) {
        await loadConfig()
        // Store update will be handled by parent component via onSave callback
        onSave()
        return true
      }
      error = response.data.message
      return false
    } catch (err: any) {
      console.error('Failed to save agent config:', err)
      error =
        err.response?.data?.error ||
        err.response?.data?.message ||
        err.message ||
        'Failed to save agent config'
      return false
    }
  }

  const handleApplyGroup = (toolTypes: string[]) => {
    enabledTools = Array.from(new Set([...enabledTools, ...toolTypes]))
    persistConfig()
  }

  const handleRemoveGroup = (toolTypes: string[]) => {
    enabledTools = enabledTools.filter((tool) => !toolTypes.includes(tool))
    persistConfig()
  }

  const handleClearAll = () => {
    enabledTools = []
    persistConfig()
  }

  const handleSave = async () => {
    savingConfig = true
    const success = await persistConfig()
    savingConfig = false

    if (success) {
      onClose()
    }
  }
</script>

<div class="config-panel" class:visible={isOpen}>
  <div class="config-header">
    <div style="display: flex; align-items: center; gap: 0.75rem;">
      <MaterialIcon name="robot-confused" width="28" height="28" />
      <h4>Agent Configuration</h4>
    </div>
    <button class="close-button" onclick={onClose} aria-label="Close">
      ✕
    </button>
  </div>
  <div class="config-content">
    {#if error}
      <div class="error">{error}</div>
    {/if}

    <div style="margin-bottom: 2rem;">
      <CheckboxWithHelp
        bind:checked={debugLogging}
        label="Debug Conversation Logging"
        helpText="Writes detailed logs of the agent conversation (system prompts, thinking, tool calls, results) to a single file in public/logs per conversation. Useful for debugging agent behavior."
      />
    </div>

    <div class="tab-bar">
      <div class="tab-buttons">
        <Button
          variant={activeTab === 'tools' ? 'primary' : 'secondary'}
          size="small"
          onclick={() => (activeTab = 'tools')}
        >
          Tools
        </Button>
        <Button
          variant={activeTab === 'groups' ? 'primary' : 'secondary'}
          size="small"
          onclick={() => (activeTab = 'groups')}
        >
          Groups
        </Button>
      </div>
      <Button
        variant="ghost"
        size="small"
        onclick={handleClearAll}
        disabled={enabledTools.length === 0}
      >
        Clear All
      </Button>
    </div>

    {#if activeTab === 'tools'}
      <ToolsConfigSection {enabledTools} onToggle={handleToolToggle} />
    {:else}
      <ToolGroupsConfigSection
        {enabledTools}
        onApply={handleApplyGroup}
        onRemove={handleRemoveGroup}
      />
    {/if}
  </div>
  <div class="config-footer">
    <Button variant="secondary" onclick={onClose}>Cancel</Button>
    <Button variant="primary" onclick={handleSave} disabled={savingConfig}>
      {savingConfig ? 'Saving...' : 'Save'}
    </Button>
  </div>
</div>

<style>
  .config-panel {
    width: 60%;
    height: 100%;
    background-color: var(--bg-primary, #fff);
    transform: translateX(100%);
    transition:
      transform 0.3s ease-in-out,
      background-color 0.3s ease,
      border-color 0.3s ease;
    z-index: 10;
    display: flex;
    flex-direction: column;
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    max-height: 100vh;
  }

  .config-panel.visible {
    transform: translateX(0);
  }

  .config-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--border-color, #e0e0e0);
    background-color: var(--bg-secondary, #f9f9f9);
    border-top-left-radius: 8px;
    border-bottom-left-radius: 8px;
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease;
  }

  .config-header h4 {
    margin: 0;
    color: var(--text-primary, #100f0f);
    font-size: 1.2rem;
    font-weight: 600;
    transition: color 0.3s ease;
    border-radius: 8px;
  }

  .close-button {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: var(--text-secondary, #666);
    padding: 0;
    width: 2rem;
    height: 2rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    transition:
      background-color 0.2s,
      color 0.3s ease;
  }

  .close-button:hover {
    background-color: var(--bg-tertiary, #e0e0e0);
    color: var(--text-primary, #100f0f);
  }

  .config-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }

  .tab-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .tab-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .config-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 1rem;
    border-top: 1px solid var(--border-color, #e0e0e0);
    background-color: var(--bg-secondary, #f9f9f9);
    transition:
      border-color 0.3s ease,
      background-color 0.3s ease;
  }

  .error {
    padding: 0.75rem;
    margin-bottom: 1rem;
    background-color: rgba(255, 200, 200, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.5);
    border-radius: 8px;
    color: var(--accent-color, #c33);
    font-size: 0.9rem;
    transition:
      background-color 0.3s ease,
      border-color 0.3s ease,
      color 0.3s ease;
  }

  @media screen and (max-width: 768px) {
    .config-panel {
      width: 100%;
      min-width: 100%;
      max-width: 100%;
    }
  }
</style>
