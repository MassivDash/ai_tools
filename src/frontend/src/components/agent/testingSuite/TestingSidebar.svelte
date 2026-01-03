<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { axiosBackendInstance } from '@axios/axiosBackendInstance'
  import MaterialIcon from '@ui/MaterialIcon.svelte'
  import IconButton from '@ui/IconButton.svelte'
  import SidebarHeader from '@ui/SidebarHeader.svelte'
  import type { TestSuite, TestQuestion } from '@types'
  import { utils, writeFile } from 'xlsx'

  import SuiteList from './components/SuiteList.svelte'
  import QuestionList from './components/QuestionList.svelte'
  import TestRunnerControls from './components/TestRunnerControls.svelte'

  export let isOpen = false

  const dispatch = createEventDispatcher<{
    runQuestion: { content: string }
    copyQuestion: { content: string }
    close: void
  }>()

  let suites: TestSuite[] = []
  let questions: TestQuestion[] = []
  let selectedSuite: TestSuite | null = null
  let error = ''

  // Refs
  let suiteListInfo: SuiteList

  // Test Runner State
  let running = false
  let currentQuestionIndex = -1
  let runStatus: 'idle' | 'running' | 'completed' = 'idle'

  // Metrics
  let startTime: number | null = null
  let endTime: number | null = null
  let totalTokens = 0
  let totalChars = 0

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
    }
  }

  const handleBackToSuites = () => {
    selectedSuite = null
    questions = []
    runStatus = 'idle'
  }

  // --- Suite CRUD ---
  const handleSaveSuite = async (
    e: CustomEvent<{ id: string; name: string; description: string }>
  ) => {
    const { id, name, description } = e.detail
    try {
      await axiosBackendInstance.put(`agent/testing/suites/${id}`, {
        name,
        description
      })
      // Update local state
      suites = suites.map((s) =>
        s.id === id ? { ...s, name, description } : s
      )
      if (selectedSuite && selectedSuite.id === id) {
        selectedSuite = { ...selectedSuite, name, description }
      }
    } catch {
      error = 'Failed to update suite name'
    }
  }

  const handleCreateSuite = async (
    e: CustomEvent<{ name: string; description: string }>
  ) => {
    const { name, description } = e.detail
    try {
      await axiosBackendInstance.post('agent/testing/suites', {
        name,
        description
      })
      await loadSuites()
    } catch {
      error = 'Failed to create suite'
    }
  }

  const handleDeleteSuite = async (e: CustomEvent<{ id: string }>) => {
    try {
      await axiosBackendInstance.delete(`agent/testing/suites/${e.detail.id}`)
      await loadSuites()
    } catch {
      error = 'Failed to delete suite'
    }
  }

  // --- Question CRUD ---
  const handleSaveQuestion = async (
    e: CustomEvent<{ id?: number; content: string }>
  ) => {
    if (!selectedSuite) return
    const { content } = e.detail
    try {
      await axiosBackendInstance.post(
        `agent/testing/suites/${selectedSuite.id}/questions`,
        { content }
      )
      await loadQuestions(selectedSuite)
    } catch {
      error = 'Failed to save question'
    }
  }

  const handleUpdateQuestion = async (
    e: CustomEvent<{ id: number; content: string }>
  ) => {
    const { id, content } = e.detail
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

  const handleDeleteQuestion = async (e: CustomEvent<{ id: number }>) => {
    try {
      await axiosBackendInstance.delete(
        `agent/testing/questions/${e.detail.id}`
      )
      if (selectedSuite) await loadQuestions(selectedSuite)
    } catch {
      error = 'Failed to delete question'
    }
  }

  // --- Import/Export (Logic moved from component but driven by events) ---
  const handleImportQuestions = async (
    e: CustomEvent<{ questions: string[] }>
  ) => {
    if (!selectedSuite) return
    const newQuestions = e.detail.questions
    try {
      // Batch add logic
      for (const content of newQuestions) {
        await axiosBackendInstance.post(
          `agent/testing/suites/${selectedSuite.id}/questions`,
          { content }
        )
      }
      await loadQuestions(selectedSuite)
      error = ''
    } catch (err: any) {
      console.error('Import processing failed', err)
      error = err.message || 'Failed to import file'
    }
  }

  const handleExportQuestions = () => {
    if (questions.length === 0) return

    const data = questions.map((q) => ({ questions: q.content }))
    const worksheet = utils.json_to_sheet(data)
    const workbook = utils.book_new()
    utils.book_append_sheet(workbook, worksheet, 'Questions')

    const filename = selectedSuite?.name
      ? `${selectedSuite.name.replace(/[^a-z0-9]/gi, '_').toLowerCase()}_questions.xlsx`
      : 'testing_questions.xlsx'

    writeFile(workbook, filename)
  }

  // --- Runner Logic ---
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

  // QuestionList Ref
  let questionList: QuestionList
</script>

<div class="testing-sidebar" class:open={isOpen}>
  <div class="header-container">
    {#if selectedSuite}
      <SidebarHeader
        title={selectedSuite.name}
        showAdd={true}
        addTitle="Add Question"
        on:add={() => questionList?.openAddQuestionForm()}
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
        showAdd={true}
        addTitle="New Suite"
        on:add={() => suiteListInfo?.openNewSuiteForm()}
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
      <SuiteList
        bind:this={suiteListInfo}
        {suites}
        selectedSuiteId={selectedSuite?.id || null}
        on:select={(e) => loadQuestions(e.detail.suite)}
        on:save={handleSaveSuite}
        on:create={handleCreateSuite}
        on:delete={handleDeleteSuite}
      />
    {:else}
      <TestRunnerControls
        {running}
        {runStatus}
        questionsCount={questions.length}
        {currentQuestionIndex}
        {totalTokens}
        {startTime}
        {endTime}
        {totalChars}
        on:start={startRunner}
        on:stop={stopRunner}
        on:import={handleImportQuestions}
        on:export={handleExportQuestions}
        on:error={(e) => (error = e.detail.message)}
      />

      <QuestionList
        bind:this={questionList}
        {questions}
        {currentQuestionIndex}
        {running}
        on:save={handleSaveQuestion}
        on:update={handleUpdateQuestion}
        on:delete={handleDeleteQuestion}
        on:copy={(e) => dispatch('copyQuestion', { content: e.detail.content })}
      />
    {/if}
  </div>
</div>

<style>
  .testing-sidebar {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    width: 320px;
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

  .error {
    color: red;
    padding: 1rem;
    font-size: 0.9rem;
  }
</style>
