<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance'
  import MaterialIcon from '../ui/MaterialIcon.svelte'
  import Button from '../ui/Button.svelte'
  import IconButton from '../ui/IconButton.svelte'
  import SidebarHeader from '../ui/SidebarHeader.svelte'
  import EditableListItem from '../ui/EditableListItem.svelte'
  import Input from '../ui/Input.svelte'
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
  let showForm = false

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
    showForm = false
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
      loadSuites()
      suiteName = ''
      suiteDescription = ''
      editingSuiteId = null
      showForm = false
    } catch {
      // ignore
    }
  }

  const updateSuiteName = async (
    id: string,
    name: string,
    description: string
  ) => {
    try {
      await axiosBackendInstance.put(`agent/testing/suites/${id}`, {
        name,
        description
      })
      // Update local state
      suites = suites.map((s) => (s.id === id ? { ...s, name } : s))
      if (selectedSuite && selectedSuite.id === id) {
        selectedSuite = { ...selectedSuite, name }
      }
    } catch {
      error = 'Failed to update suite name'
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

  const cancelEditSuite = () => {
    editingSuiteId = null
    suiteName = ''
    suiteDescription = ''
    showForm = false
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
      showForm = false
    } catch {
      error = 'Failed to save question'
    }
  }

  const updateQuestionContent = async (id: number, content: string) => {
    try {
      await axiosBackendInstance.put(`agent/testing/questions/${id}`, {
        content
      })
      // Update local state
      questions = questions.map((q) => (q.id === id ? { ...q, content } : q))
    } catch {
      error = 'Failed to update question'
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

  const cancelEditQuestion = () => {
    editingQuestionId = null
    questionContent = ''
    showForm = false
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
    runStatus = 'idle'
  }

  $: if (isOpen) {
    if (!selectedSuite) loadSuites()
  }
</script>

<div class="testing-sidebar" class:open={isOpen}>
  <div class="header-container">
    {#if selectedSuite}
      <SidebarHeader
        title={selectedSuite.name}
        showAdd={!showForm}
        addTitle="Add Question"
        on:add={() => (showForm = true)}
        on:close={() => dispatch('close')}
        closeTitle="Close Testing"
      >
        <div slot="prefix">
          <IconButton
            variant="ghost"
            class="back-btn sidebar-icon-btn"
            onclick={handleBackToSuites}
            title="Back to Suites"
            iconSize={20}
          >
            <MaterialIcon name="arrow-left" width="20" height="20" />
          </IconButton>
        </div>
      </SidebarHeader>
    {:else}
      <SidebarHeader
        title="Auto Testing"
        icon="flask"
        showAdd={!showForm}
        addTitle="New Suite"
        on:add={() => (showForm = true)}
        on:close={() => dispatch('close')}
        closeTitle="Close Testing"
      />
    {/if}
  </div>

  <div class="content">
    {#if error}
      <div class="error">{error}</div>
    {/if}

    {#if !selectedSuite}
      <!-- Suites List -->
      {#if showForm}
        <div class="form-section">
          <Input type="text" placeholder="Suite Name" bind:value={suiteName} />
          <Input
            type="text"
            placeholder="Description"
            bind:value={suiteDescription}
          />
          <div class="form-actions">
            <Button variant="primary" onclick={saveSuite} disabled={!suiteName}>
              {editingSuiteId ? 'Update' : 'Create'} Suite
            </Button>
            <Button variant="secondary" onclick={cancelEditSuite}>
              Cancel
            </Button>
          </div>
        </div>
      {/if}

      <div class="list">
        {#each suites as suite (suite.id)}
          <EditableListItem
            title={suite.name}
            active={selectedSuite?.id === suite.id}
            on:click={() => loadQuestions(suite)}
            on:save={(e) =>
              updateSuiteName(suite.id, e.detail, suite.description || '')}
            on:delete={() => deleteSuite(suite.id)}
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

      {#if showForm}
        <div class="form-section">
          <textarea
            placeholder="Question content..."
            bind:value={questionContent}
          ></textarea>
          <div class="form-actions">
            <Button
              variant="primary"
              onclick={saveQuestion}
              disabled={!questionContent}
            >
              {editingQuestionId ? 'Update' : 'Add'} Question
            </Button>
            <Button variant="secondary" onclick={cancelEditQuestion}>
              Cancel
            </Button>
          </div>
        </div>
      {/if}

      <div class="list">
        {#each questions as q, i (q.id)}
          <EditableListItem
            title={q.content}
            active={i === currentQuestionIndex && running}
            on:save={(e) => updateQuestionContent(q.id, e.detail)}
            on:delete={() => deleteQuestion(q.id)}
          >
            <div style="display: flex; gap: 8px;">
              <span class="index">{i + 1}.</span>
              <span class="content-text">{q.content}</span>
            </div>
          </EditableListItem>
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
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
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

  .content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

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

  .runner-controls {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    align-items: center;
    gap: 1rem;
    border-bottom: 1px solid var(--border-color);
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
    color: var(--text-secondary);
  }
</style>
