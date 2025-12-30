<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import Button from '../ui/Button.svelte'
  import type { TestSuite, TestQuestion } from './types'

  export let isOpen = false

  const dispatch = createEventDispatcher<{
    runQuestion: { content: string }
    close: void
  }>()

  let suites: TestSuite[] = []
  let questions: TestQuestion[] = []
  let selectedSuite: TestSuite | null = null
  let error = ''

  // Suite Form
  let editingSuiteId: string | null = null
  let suiteName = ''
  let suiteDescription = ''

  // Question Form
  let editingQuestionId: number | null = null
  let questionContent = ''

  // Test Runner State
  let running = false
  let currentQuestionIndex = -1
  let runStatus: 'idle' | 'running' | 'completed' = 'idle'

  // Load Suites
  const loadSuites = async () => {
    error = ''
    try {
      const response = await axiosBackendInstance.get<TestSuite[]>(
        'agent/testing/suites'
      )
      suites = response.data
    } catch {
      error = 'Failed to load test suites'
    } finally {
      // loading not used
    }
  }

  // Load Questions
  const loadQuestions = async (suite: TestSuite) => {
    selectedSuite = suite
    try {
      const response = await axiosBackendInstance.get<TestQuestion[]>(
        `agent/testing/suites/${suite.id}/questions`
      )
      questions = response.data
    } catch {
      error = 'Failed to load questions'
    } finally {
      // loading not used
    }
  }

  const handleBackToSuites = () => {
    selectedSuite = null
    questions = []
    runStatus = 'idle'
  }

  // --- Suite CRUD ---
  const saveSuite = async () => {
    if (!suiteName.trim()) return

    try {
      if (editingSuiteId) {
        await axiosBackendInstance.put(
          `agent/testing/suites/${editingSuiteId}`,
          {
            name: suiteName,
            description: suiteDescription
          }
        )
      } else {
        await axiosBackendInstance.post('agent/testing/suites', {
          name: suiteName,
          description: suiteDescription
        })
      }
      loadSuites()
      suiteName = ''
      suiteDescription = ''
      editingSuiteId = null
    } catch {
      error = 'Failed to save suite'
    }
  }

  const deleteSuite = async (id: string) => {
    try {
      await axiosBackendInstance.delete(`agent/testing/suites/${id}`)
      loadSuites()
    } catch {
      error = 'Failed to delete suite'
    }
  }

  const startEditSuite = (suite: TestSuite) => {
    editingSuiteId = suite.id
    suiteName = suite.name
    suiteDescription = suite.description || ''
  }

  const cancelEditSuite = () => {
    editingSuiteId = null
    suiteName = ''
    suiteDescription = ''
  }

  // --- Question CRUD ---
  const saveQuestion = async () => {
    if (!selectedSuite || !questionContent.trim()) return

    try {
      if (editingQuestionId) {
        await axiosBackendInstance.put(
          `agent/testing/questions/${editingQuestionId}`,
          {
            content: questionContent
          }
        )
      } else {
        await axiosBackendInstance.post(
          `agent/testing/suites/${selectedSuite.id}/questions`,
          {
            content: questionContent
          }
        )
      }
      loadQuestions(selectedSuite)
      questionContent = ''
      editingQuestionId = null
    } catch {
      error = 'Failed to save question'
    }
  }

  const deleteQuestion = async (id: number) => {
    try {
      await axiosBackendInstance.delete(`agent/testing/questions/${id}`)
      if (selectedSuite) loadQuestions(selectedSuite)
    } catch {
      error = 'Failed to delete question'
    }
  }

  const startEditQuestion = (q: TestQuestion) => {
    editingQuestionId = q.id
    questionContent = q.content
  }

  const cancelEditQuestion = () => {
    editingQuestionId = null
    questionContent = ''
  }

  // --- Test Runner ---
  // Metrics
  let startTime: number | null = null
  let endTime: number | null = null
  let totalTokens = 0
  let totalChars = 0

  export const handleRunnerNext = () => {
    // Called by parent when ready for next question (triggered by loading=false debounce)
    if (running && currentQuestionIndex < questions.length - 1) {
      currentQuestionIndex++
      dispatch('runQuestion', {
        content: questions[currentQuestionIndex].content
      })
    } else if (running) {
      running = false
      endTime = Date.now()
      runStatus = 'completed'
    }
  }

  export const handleResponseMetrics = (metrics: {
    usage: any
    content: string
  }) => {
    if (running) {
      if (metrics.usage) {
        totalTokens += metrics.usage.total_tokens || 0
      }
      if (metrics.content) {
        totalChars += metrics.content.length
      }
    }
  }

  const startRunner = () => {
    if (questions.length === 0) return
    running = true
    runStatus = 'running'
    currentQuestionIndex = 0
    startTime = Date.now()
    endTime = null
    totalTokens = 0
    totalChars = 0
    dispatch('runQuestion', { content: questions[0].content })
  }

  const stopRunner = () => {
    running = false
    endTime = Date.now()
    runStatus = 'idle' // Or keep it running if just paused? No, stop means stop.
  }

  $: if (isOpen) {
    if (!selectedSuite) loadSuites()
  }
</script>

<div class="testing-sidebar" class:open={isOpen}>
  <div class="header">
    <div class="header-left">
      {#if selectedSuite}
        <Button
          variant="info"
          class="back-btn sidebar-icon-btn button-icon-only"
          onclick={handleBackToSuites}
          title="Back to Suites"
        >
          <MaterialIcon name="arrow-left" width="20" height="20" />
        </Button>
        <h2 class="suite-title">{selectedSuite.name}</h2>
      {:else}
        <div style="display: flex; align-items: center; gap: 0.5rem;">
          <MaterialIcon name="flask" width="20" height="20" />
          <h2>Auto Testing</h2>
        </div>
      {/if}
    </div>
    <div class="actions">
      <Button
        variant="info"
        class="sidebar-icon-btn button-icon-only"
        onclick={() => dispatch('close')}
        title="Close Testing"
      >
        <MaterialIcon name="chevron-left" width="20" height="20" />
      </Button>
    </div>
  </div>

  <div class="content">
    {#if error}
      <div class="error">{error}</div>
    {/if}

    {#if !selectedSuite}
      <!-- Suites List -->
      <div class="form-section">
        <input type="text" placeholder="Suite Name" bind:value={suiteName} />
        <input
          type="text"
          placeholder="Description"
          bind:value={suiteDescription}
        />
        <div class="form-actions">
          <Button variant="primary" onclick={saveSuite} disabled={!suiteName}>
            {editingSuiteId ? 'Update' : 'Create'} Suite
          </Button>
          {#if editingSuiteId}
            <Button variant="secondary" onclick={cancelEditSuite}>Cancel</Button
            >
          {/if}
        </div>
      </div>

      <div class="list">
        {#each suites as suite (suite.id)}
          <div
            class="item"
            on:click={() => loadQuestions(suite)}
            role="button"
            tabindex="0"
            on:keypress={(e) => e.key === 'Enter' && loadQuestions(suite)}
          >
            <div class="info">
              <span class="name">{suite.name}</span>
              <span class="desc">{suite.description || ''}</span>
            </div>
            <div class="item-actions">
              <button
                on:click|stopPropagation={() => startEditSuite(suite)}
                title="Edit"
              >
                <MaterialIcon name="pencil" width="16" height="16" />
              </button>
              <button
                class="delete"
                on:click|stopPropagation={() => deleteSuite(suite.id)}
                title="Delete"
              >
                <MaterialIcon name="delete" width="16" height="16" />
              </button>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <!-- Questions List -->
      <div class="runner-controls">
        {#if running}
          <div class="running-state">
            <Button variant="danger" onclick={stopRunner} title="Stop Testing">
              <MaterialIcon name="stop" width="20" height="20" /> Stop
            </Button>
            <div class="running-indicator">
              Running ({currentQuestionIndex + 1}/{questions.length})
            </div>
          </div>
        {:else}
          <Button
            variant="primary"
            onclick={startRunner}
            disabled={questions.length === 0}
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
      </div>

      <div class="form-section">
        <textarea placeholder="Question content..." bind:value={questionContent}
        ></textarea>
        <div class="form-actions">
          <Button
            variant="primary"
            onclick={saveQuestion}
            disabled={!questionContent}
          >
            {editingQuestionId ? 'Update' : 'Add'} Question
          </Button>
          {#if editingQuestionId}
            <Button variant="secondary" onclick={cancelEditQuestion}
              >Cancel</Button
            >
          {/if}
        </div>
      </div>

      <div class="list">
        {#each questions as q, i (q.id)}
          <div
            class="item question-item"
            class:active={i === currentQuestionIndex && running}
          >
            <span class="index">{i + 1}.</span>
            <span class="content-text">{q.content}</span>
            <div class="item-actions">
              <button
                on:click|stopPropagation={() => startEditQuestion(q)}
                title="Edit"
              >
                <MaterialIcon name="pencil" width="16" height="16" />
              </button>
              <button
                class="delete"
                on:click|stopPropagation={() => deleteQuestion(q.id)}
                title="Delete"
              >
                <MaterialIcon name="delete" width="16" height="16" />
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .testing-sidebar {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    width: 320px; /* Slightly wider for questions */
    background: var(--bg-secondary, #f5f5f5);
    border-right: 1px solid var(--border-color, #e0e0e0);
    transform: translateX(-100%);
    transition: transform 0.3s ease;
    border-top-right-radius: 8px;
    border-bottom-right-radius: 8px;
    z-index: 20;
    display: flex;
    flex-direction: column;
  }

  .testing-sidebar.open {
    transform: translateX(0);
  }

  .header {
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-color, #e0e0e0);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
  }

  .header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary, #333);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .suite-title {
    font-size: 1rem !important;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .form-section {
    padding: 1rem;
    border-bottom: 1px solid var(--border-light, #eee);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  input,
  textarea {
    padding: 8px;
    border: 1px solid var(--border-color, #ccc);
    border-radius: 4px;
    font-family: inherit;
  }

  textarea {
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

  .item {
    padding: 0.75rem 1rem;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-light, #eee);
    transition: background 0.2s;
  }

  .item:hover {
    background-color: var(--bg-tertiary, #fafafa);
  }

  .item.question-item {
    cursor: default;
    align-items: flex-start;
  }

  .item.active {
    background-color: rgba(33, 150, 243, 0.1);
    border-left: 3px solid var(--primary-color, #2196f3);
  }

  .info {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .name {
    font-weight: 500;
    color: var(--text-primary, #333);
  }

  .desc {
    font-size: 0.8rem;
    color: var(--text-secondary, #666);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .index {
    font-weight: bold;
    color: var(--text-secondary, #999);
    margin-right: 8px;
    min-width: 20px;
  }

  .content-text {
    flex: 1;
    font-size: 0.9rem;
    color: var(--text-primary, #333);
    word-break: break-word;
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

  .item-actions button {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
    color: var(--text-secondary, #999);
    display: flex;
  }

  .item-actions button:hover {
    color: var(--primary-color, #2196f3);
  }

  .item-actions button.delete:hover {
    color: #f44336;
  }

  .runner-controls {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    border-bottom: 1px solid var(--border-light, #eee);
  }

  .running-state {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .metrics-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    /* No extra padding/border needed here as parent has gap/padding */
  }

  .metrics-details {
    display: flex;
    gap: 8px;
    font-size: 0.8rem;
    color: var(--text-secondary, #666);
  }
</style>
