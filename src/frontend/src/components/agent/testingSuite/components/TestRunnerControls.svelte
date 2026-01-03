<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import Button from '@ui/Button.svelte'
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  import { parseQuestionsFromFile } from '../../utils/testingUtils'

  export let running = false
  export let questionsCount = 0
  export let currentQuestionIndex = 0
  export let runStatus: 'idle' | 'running' | 'completed' = 'idle'

  // Metrics
  export let totalTokens = 0
  export let startTime: number | null = null
  export let endTime: number | null = null
  export let totalChars = 0

  const dispatch = createEventDispatcher<{
    start: void
    stop: void
    import: { questions: string[] }
    export: void
    error: { message: string }
  }>()

  let fileInput: HTMLInputElement

  const handleImportClick = () => {
    fileInput.click()
  }

  const handleFileChange = async (e: Event) => {
    const target = e.target as HTMLInputElement
    if (!target.files || target.files.length === 0) return

    const file = target.files[0]
    try {
      const newQuestions = await parseQuestionsFromFile(file)
      dispatch('import', { questions: newQuestions })
    } catch (err: any) {
      console.error('Import failed', err)
      dispatch('error', { message: err.message || 'Failed to import file' })
    } finally {
      target.value = ''
    }
  }
</script>

<div class="runner-controls">
  {#if running}
    <div class="running-state">
      <Button
        variant="danger"
        onclick={() => dispatch('stop')}
        title="Stop Testing"
      >
        <MaterialIcon name="stop" width="20" height="20" /> Stop
      </Button>
      <div class="running-indicator">
        Running ({currentQuestionIndex + 1}/{questionsCount})
      </div>
    </div>
  {:else}
    <Button
      variant="primary"
      onclick={() => dispatch('start')}
      disabled={questionsCount === 0}
    >
      <MaterialIcon name="play" width="20" height="20" /> Run Suite
    </Button>
  {/if}

  {#if runStatus === 'completed'}
    <div class="metrics-container">
      <div class="completed-badge-enhanced">
        <MaterialIcon name="check-circle" width="18" height="18" />
        <span>Done in {((endTime || 0) - (startTime || 0)) / 1000}s</span>
      </div>
      <div class="metrics-details">
        <span>{totalTokens} tokens</span>
        <span>{totalChars} chars</span>
      </div>
    </div>
  {/if}

  {#if !running}
    <div class="io-controls">
      <input
        type="file"
        accept=".xlsx, .xls, .csv"
        style="display: none;"
        bind:this={fileInput}
        onchange={handleFileChange}
      />
      <Button
        variant="secondary"
        onclick={handleImportClick}
        title="Import from Excel"
      >
        <MaterialIcon name="upload" width="18" height="18" /> Import
      </Button>
      <Button
        variant="secondary"
        onclick={() => dispatch('export')}
        disabled={questionsCount === 0}
        title="Export to Excel"
      >
        <MaterialIcon name="download" width="18" height="18" /> Export
      </Button>
    </div>
  {/if}
</div>

<style>
  .runner-controls {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .running-state {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .running-indicator {
    font-size: 0.9rem;
    color: var(--text-primary);
  }

  .metrics-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .metrics-details {
    display: flex;
    gap: 8px;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .completed-badge-enhanced {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--success, #4caf50);
    font-weight: 500;
  }

  .io-controls {
    display: flex;
    gap: 0.5rem;
    width: 100%;
    justify-content: center;
  }
</style>
