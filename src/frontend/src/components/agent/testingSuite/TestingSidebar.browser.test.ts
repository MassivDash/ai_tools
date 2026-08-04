/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import {
  render,
  screen,
  fireEvent,
  waitFor,
  within
} from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import TestingSidebar from './TestingSidebar.svelte'
import { axiosBackendInstance } from '../../../axiosInstance/axiosBackendInstance'
import { parseQuestionsFromFile } from '../utils/testingUtils'
import { utils, writeFile } from 'xlsx'
import type { Component } from 'svelte'
import type { TestQuestion, TestSuite } from '@types'

// Mock axiosBackendInstance
vi.mock('../../../axiosInstance/axiosBackendInstance', () => ({
  axiosBackendInstance: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
    defaults: { baseURL: 'http://localhost:8000' }
  }
}))

// The sidebar only orchestrates xlsx / file parsing, so both are stubbed here.
vi.mock('xlsx', () => ({
  read: vi.fn(),
  writeFile: vi.fn(),
  utils: {
    json_to_sheet: vi.fn(() => ({ kind: 'worksheet' })),
    book_new: vi.fn(() => ({ kind: 'workbook' })),
    book_append_sheet: vi.fn(),
    sheet_to_json: vi.fn(() => [])
  }
}))

vi.mock('../utils/testingUtils', () => ({
  parseQuestionsFromFile: vi.fn()
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
  put: ReturnType<typeof vi.fn>
  delete: ReturnType<typeof vi.fn>
}

const mockedParse = parseQuestionsFromFile as unknown as ReturnType<
  typeof vi.fn
>
const mockedWriteFile = writeFile as unknown as ReturnType<typeof vi.fn>
const mockedUtils = utils as unknown as {
  json_to_sheet: ReturnType<typeof vi.fn>
  book_new: ReturnType<typeof vi.fn>
  book_append_sheet: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(console, 'log').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('loads suites on open', async () => {
  const suites = [
    {
      id: '1',
      name: 'Suite 1',
      description: 'Test Suite 1',
      created_at: Date.now()
    },
    {
      id: '2',
      name: 'Suite 2',
      description: 'Test Suite 2',
      created_at: Date.now()
    }
  ]
  mockedAxios.get.mockResolvedValue({ data: suites })

  render(TestingSidebar as Component, {
    props: { isOpen: true }
  })

  await waitFor(() => {
    expect(screen.getByText('Suite 1')).toBeTruthy()
    expect(screen.getByText('Suite 2')).toBeTruthy()
  })

  expect(mockedAxios.get).toHaveBeenCalledWith('agent/testing/suites')
})

test('creates a new suite', async () => {
  mockedAxios.get.mockResolvedValueOnce({ data: [] })
  mockedAxios.post.mockResolvedValue({ data: { success: true } })
  mockedAxios.get.mockResolvedValueOnce({
    data: [
      {
        id: '1',
        name: 'New Suite',
        description: 'Desc',
        created_at: Date.now()
      }
    ]
  })

  render(TestingSidebar as Component, {
    props: { isOpen: true }
  })

  const addBtn = screen.getByTitle('New Suite')
  await fireEvent.click(addBtn)

  const input = screen.getByPlaceholderText('Suite Name')
  await fireEvent.input(input, { target: { value: 'New Suite' } })

  const createBtn = screen.getByText('Create Suite')
  await fireEvent.click(createBtn)

  expect(mockedAxios.post).toHaveBeenCalledWith('agent/testing/suites', {
    name: 'New Suite',
    description: ''
  })

  await waitFor(() => {
    expect(screen.getByText(/New Suite/)).toBeTruthy()
  })
})

test('loads questions when clicking a suite', async () => {
  const suites = [
    {
      id: '1',
      name: 'Suite 1',
      description: 'Test Suite 1',
      created_at: Date.now()
    }
  ]
  const questions = [
    {
      id: '101',
      suite_id: '1',
      content: 'What is the capital of France?',
      created_at: Date.now()
    }
  ]

  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/testing/suites') return Promise.resolve({ data: suites })
    if (url === 'agent/testing/suites/1/questions')
      return Promise.resolve({ data: questions })
    return Promise.resolve({ data: [] })
  })

  render(TestingSidebar as Component, {
    props: { isOpen: true }
  })

  await waitFor(() => {
    expect(screen.getByText('Suite 1')).toBeTruthy()
  })

  await waitFor(() => {
    expect(screen.getByText('Suite 1')).toBeTruthy()
  })

  // EditableListItem renders title in a span, we can find it by text and click it
  const suiteText = screen.getByText('Suite 1')
  await fireEvent.click(suiteText)

  await waitFor(() => {
    expect(screen.getByText('What is the capital of France?')).toBeTruthy()
  })

  expect(screen.getByText('Suite 1')).toBeTruthy()
})

test('runs questions sequentially', async () => {
  // We verify that clicking 'Run Suite' works

  const questions = [
    { id: '101', suite_id: '1', content: 'Question 1', created_at: Date.now() },
    { id: '102', suite_id: '1', content: 'Question 2', created_at: Date.now() }
  ]
  const suites = [{ id: '1', name: 'Suite 1', created_at: Date.now() }]

  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/testing/suites') return Promise.resolve({ data: suites })
    if (url === 'agent/testing/suites/1/questions')
      return Promise.resolve({ data: questions })
    return Promise.resolve({ data: [] })
  })

  const { component } = render(TestingSidebar as Component, {
    props: { isOpen: true }
  })

  await waitFor(() => expect(screen.getByText('Suite 1')).toBeTruthy())

  await waitFor(() => expect(screen.getByText('Suite 1')).toBeTruthy())
  const suiteItem = screen.getByText('Suite 1').closest('.item')
  if (suiteItem) await fireEvent.click(suiteItem)

  await waitFor(() => expect(screen.getByText('Question 1')).toBeTruthy())

  // Click Run
  const runBtn = screen.getByRole('button', { name: /Run Suite/i })
  await fireEvent.click(runBtn)

  await waitFor(() => {
    // Check if button text changed
    expect(screen.getByText(/Running \(1\/2\)/)).toBeTruthy()
    // Check if first question is active
    const q1 = screen.getByText('Question 1').closest('.item')
    expect(q1?.classList.contains('active')).toBe(true)
  })

  // Call internal method to simulate next question
  ;(component as any).handleRunnerNext()

  await waitFor(() => {
    // Check if button text updated
    expect(screen.getByText(/Running \(2\/2\)/)).toBeTruthy()
    // Check if second question is active
    const q2 = screen.getByText('Question 2').closest('.item')
    expect(q2?.classList.contains('active')).toBe(true)
  })
})

test('runs 4 questions without skipping', async () => {
  const questions = [
    { id: '101', suite_id: '1', content: 'Q1', created_at: Date.now() },
    { id: '102', suite_id: '1', content: 'Q2', created_at: Date.now() },
    { id: '103', suite_id: '1', content: 'Q3', created_at: Date.now() },
    { id: '104', suite_id: '1', content: 'Q4', created_at: Date.now() }
  ]
  const suites = [{ id: '1', name: 'Suite 1', created_at: Date.now() }]

  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/testing/suites') return Promise.resolve({ data: suites })
    if (url === 'agent/testing/suites/1/questions')
      return Promise.resolve({ data: questions })
    return Promise.resolve({ data: [] })
  })

  const { component } = render(TestingSidebar as Component, {
    props: { isOpen: true }
  })

  await waitFor(() => expect(screen.getByText('Suite 1')).toBeTruthy())
  const suiteItem = screen.getByText('Suite 1').closest('.item')
  if (suiteItem) await fireEvent.click(suiteItem)
  await waitFor(() => expect(screen.getByText('Q1')).toBeTruthy())

  // Click Run - Start (Q1)
  const runBtn = screen.getByRole('button', { name: /Run Suite/i })
  await fireEvent.click(runBtn)

  await waitFor(() => {
    const q1 = screen.getByText('Q1').closest('.item')
    expect(q1?.classList.contains('active')).toBe(true)
  })

  // Next -> Q2
  ;(component as any).handleRunnerNext()
  await waitFor(() => {
    const q2 = screen.getByText('Q2').closest('.item')
    expect(q2?.classList.contains('active')).toBe(true)
  })

  // Next -> Q3 (This is the reported skip point)
  ;(component as any).handleRunnerNext()
  await waitFor(() => {
    const q3 = screen.getByText('Q3').closest('.item')
    expect(q3?.classList.contains('active')).toBe(true)
  })

  // Next -> Q4
  ;(component as any).handleRunnerNext()
  await waitFor(() => {
    const q4 = screen.getByText('Q4').closest('.item')
    expect(q4?.classList.contains('active')).toBe(true)
  })
})

// ---------------------------------------------------------------------------
// Helpers for the cases below
// ---------------------------------------------------------------------------

const SUITE: TestSuite = {
  id: '1',
  name: 'Suite 1',
  description: 'Desc',
  created_at: 1
}

const question = (id: number, content: string): TestQuestion => ({
  id,
  suite_id: SUITE.id,
  content,
  created_at: id
})

/** Backend state that survives reloads, so POST/DELETE + refetch can be asserted. */
const wireBackend = (state: {
  suites: TestSuite[]
  questions: TestQuestion[]
}) => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/testing/suites')
      return Promise.resolve({ data: state.suites })
    if (url === `agent/testing/suites/${SUITE.id}/questions`)
      return Promise.resolve({ data: state.questions })
    return Promise.reject(new Error(`unexpected GET ${url}`))
  })
  return state
}

/** The submit button inside the add-question form (the header icon shares its name). */
const submitQuestionButton = () =>
  within(document.querySelector('.form-section') as HTMLElement).getByRole(
    'button',
    { name: 'Add Question' }
  )

/** Renders the sidebar, waits for the suite list, then drills into SUITE. */
const openSuite = async (
  questions: TestQuestion[],
  events: Record<string, (_e: any) => void> = {}
) => {
  const state = wireBackend({ suites: [SUITE], questions })
  const rendered = render(TestingSidebar as Component, {
    props: { isOpen: true },
    events
  })

  await waitFor(() => expect(screen.getByText('Suite 1')).toBeInTheDocument())
  await fireEvent.click(
    screen.getByText('Suite 1').closest('.item') as HTMLElement
  )
  await waitFor(() =>
    expect(screen.getByTitle('Back to Suites')).toBeInTheDocument()
  )

  return { ...rendered, state }
}

// ---------------------------------------------------------------------------
// Open / close / error states
// ---------------------------------------------------------------------------

test('does not hit the backend while the sidebar is closed', async () => {
  wireBackend({ suites: [SUITE], questions: [] })

  const { rerender } = render(TestingSidebar as Component, {
    props: { isOpen: false }
  })

  expect(mockedAxios.get).not.toHaveBeenCalled()
  expect(screen.queryByText('Suite 1')).not.toBeInTheDocument()

  await rerender({ isOpen: true })

  await waitFor(() => expect(screen.getByText('Suite 1')).toBeInTheDocument())
  expect(mockedAxios.get).toHaveBeenCalledWith('agent/testing/suites')
})

test('shows an error banner when the suite list cannot be loaded', async () => {
  mockedAxios.get.mockRejectedValue(new Error('boom'))

  const { container } = render(TestingSidebar as Component, {
    props: { isOpen: true }
  })

  await waitFor(() =>
    expect(screen.getByText('Failed to load test suites')).toBeInTheDocument()
  )
  expect(container.querySelectorAll('.item')).toHaveLength(0)
})

test('shows an error banner when questions cannot be loaded', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/testing/suites')
      return Promise.resolve({ data: [SUITE] })
    return Promise.reject(new Error('nope'))
  })

  render(TestingSidebar as Component, { props: { isOpen: true } })

  await waitFor(() => expect(screen.getByText('Suite 1')).toBeInTheDocument())
  await fireEvent.click(
    screen.getByText('Suite 1').closest('.item') as HTMLElement
  )

  await waitFor(() =>
    expect(screen.getByText('Failed to load questions')).toBeInTheDocument()
  )
  // the suite is still selected, so the runner view is shown
  expect(screen.getByTitle('Back to Suites')).toBeInTheDocument()
})

test('the close button dispatches close', async () => {
  const onClose = vi.fn()
  wireBackend({ suites: [], questions: [] })
  render(TestingSidebar as Component, {
    props: { isOpen: true },
    events: { close: onClose }
  })

  await fireEvent.click(screen.getByTitle('Close Testing'))

  expect(onClose).toHaveBeenCalledTimes(1)
})

test('the close button also dispatches close from inside a suite', async () => {
  const onClose = vi.fn()
  await openSuite([question(101, 'Q1')], { close: onClose })

  await fireEvent.click(screen.getByTitle('Close Testing'))

  expect(onClose).toHaveBeenCalledTimes(1)
  // closing is not the same as leaving the suite
  expect(screen.getByTitle('Back to Suites')).toBeInTheDocument()
})

test('going back to the suite list clears the questions and resets the run status', async () => {
  const { component } = await openSuite([question(101, 'Q1')])
  await waitFor(() => expect(screen.getByText('Q1')).toBeInTheDocument())

  await fireEvent.click(screen.getByRole('button', { name: /Run Suite/i }))
  await waitFor(() =>
    expect(screen.getByText('Running (1/1)')).toBeInTheDocument()
  )
  ;(component as any).handleRunnerNext()
  await waitFor(() => expect(screen.getByText(/Done in/)).toBeInTheDocument())

  await fireEvent.click(screen.getByTitle('Back to Suites'))

  await waitFor(() =>
    expect(
      screen.getByRole('heading', { name: 'Auto Testing' })
    ).toBeInTheDocument()
  )
  expect(screen.queryByText('Q1')).not.toBeInTheDocument()
  expect(screen.queryByText(/Done in/)).not.toBeInTheDocument()

  // re-entering the suite starts from a clean, idle runner
  await fireEvent.click(
    screen.getByText('Suite 1').closest('.item') as HTMLElement
  )
  await waitFor(() => expect(screen.getByText('Q1')).toBeInTheDocument())
  expect(screen.queryByText(/Done in/)).not.toBeInTheDocument()
})

// ---------------------------------------------------------------------------
// Suite CRUD
// ---------------------------------------------------------------------------

test('renaming a suite persists the new name and leaves the other suites alone', async () => {
  const other: TestSuite = {
    id: '2',
    name: 'Suite 2',
    description: 'Other',
    created_at: 2
  }
  wireBackend({ suites: [SUITE, other], questions: [] })
  mockedAxios.put.mockResolvedValue({ data: {} })

  render(TestingSidebar as Component, { props: { isOpen: true } })
  await waitFor(() => expect(screen.getByText('Suite 1')).toBeInTheDocument())

  await fireEvent.click(screen.getAllByTitle('Rename')[0])
  const input = screen.getByRole('textbox') as HTMLInputElement
  await fireEvent.input(input, { target: { value: 'Renamed Suite' } })
  await fireEvent.blur(input)

  expect(mockedAxios.put).toHaveBeenCalledWith('agent/testing/suites/1', {
    name: 'Renamed Suite',
    description: 'Desc'
  })
  expect(mockedAxios.put).toHaveBeenCalledTimes(1)
  await waitFor(() =>
    expect(screen.getByText('Renamed Suite')).toBeInTheDocument()
  )
  expect(screen.queryByText('Suite 1')).not.toBeInTheDocument()
  expect(screen.getByText('Suite 2')).toBeInTheDocument()
})

test('a failed suite rename surfaces an error and keeps the old name', async () => {
  wireBackend({ suites: [SUITE], questions: [] })
  mockedAxios.put.mockRejectedValue(new Error('409'))

  render(TestingSidebar as Component, { props: { isOpen: true } })
  await waitFor(() => expect(screen.getByText('Suite 1')).toBeInTheDocument())

  await fireEvent.click(screen.getByTitle('Rename'))
  const input = screen.getByRole('textbox') as HTMLInputElement
  await fireEvent.input(input, { target: { value: 'Renamed Suite' } })
  await fireEvent.blur(input)

  await waitFor(() =>
    expect(screen.getByText('Failed to update suite name')).toBeInTheDocument()
  )
  expect(screen.getByText('Suite 1')).toBeInTheDocument()
  expect(screen.queryByText('Renamed Suite')).not.toBeInTheDocument()
})

test('a failed suite creation surfaces an error', async () => {
  wireBackend({ suites: [], questions: [] })
  mockedAxios.post.mockRejectedValue(new Error('500'))

  render(TestingSidebar as Component, { props: { isOpen: true } })

  await fireEvent.click(screen.getByTitle('New Suite'))
  await fireEvent.input(screen.getByPlaceholderText('Suite Name'), {
    target: { value: 'Nope' }
  })
  await fireEvent.click(screen.getByText('Create Suite'))

  await waitFor(() =>
    expect(screen.getByText('Failed to create suite')).toBeInTheDocument()
  )
})

test('deleting a suite calls the backend and reloads the list', async () => {
  const state = wireBackend({ suites: [SUITE], questions: [] })
  mockedAxios.delete.mockImplementation(() => {
    state.suites = []
    return Promise.resolve({ data: {} })
  })

  render(TestingSidebar as Component, { props: { isOpen: true } })
  await waitFor(() => expect(screen.getByText('Suite 1')).toBeInTheDocument())

  await fireEvent.click(screen.getByTitle('Delete'))
  await fireEvent.click(screen.getByText('Yes'))

  expect(mockedAxios.delete).toHaveBeenCalledWith('agent/testing/suites/1')
  await waitFor(() =>
    expect(screen.queryByText('Suite 1')).not.toBeInTheDocument()
  )
  expect(mockedAxios.get).toHaveBeenCalledTimes(2)
})

test('a failed suite deletion surfaces an error and keeps the suite', async () => {
  wireBackend({ suites: [SUITE], questions: [] })
  mockedAxios.delete.mockRejectedValue(new Error('500'))

  render(TestingSidebar as Component, { props: { isOpen: true } })
  await waitFor(() => expect(screen.getByText('Suite 1')).toBeInTheDocument())

  await fireEvent.click(screen.getByTitle('Delete'))
  await fireEvent.click(screen.getByText('Yes'))

  await waitFor(() =>
    expect(screen.getByText('Failed to delete suite')).toBeInTheDocument()
  )
  expect(screen.getByText('Suite 1')).toBeInTheDocument()
})

// ---------------------------------------------------------------------------
// Question CRUD
// ---------------------------------------------------------------------------

test('adding a question posts it to the selected suite and reloads', async () => {
  const { state } = await openSuite([])
  mockedAxios.post.mockImplementation(() => {
    state.questions = [question(201, 'Brand new question')]
    return Promise.resolve({ data: {} })
  })

  await fireEvent.click(screen.getByTitle('Add Question'))
  const textarea = await screen.findByPlaceholderText('Question content...')
  await fireEvent.input(textarea, { target: { value: 'Brand new question' } })
  await fireEvent.click(submitQuestionButton())

  expect(mockedAxios.post).toHaveBeenCalledWith(
    'agent/testing/suites/1/questions',
    { content: 'Brand new question' }
  )
  await waitFor(() =>
    expect(screen.getByText('Brand new question')).toBeInTheDocument()
  )
})

test('a failed question save surfaces an error', async () => {
  await openSuite([])
  mockedAxios.post.mockRejectedValue(new Error('500'))

  await fireEvent.click(screen.getByTitle('Add Question'))
  const textarea = await screen.findByPlaceholderText('Question content...')
  await fireEvent.input(textarea, { target: { value: 'Will fail' } })
  await fireEvent.click(submitQuestionButton())

  await waitFor(() =>
    expect(screen.getByText('Failed to save question')).toBeInTheDocument()
  )
})

test('editing a question updates it in place without refetching', async () => {
  await openSuite([question(101, 'Old content'), question(102, 'Untouched')])
  mockedAxios.put.mockResolvedValue({ data: {} })
  const getCallsBefore = mockedAxios.get.mock.calls.length

  await fireEvent.click(screen.getAllByTitle('Rename')[0])
  const input = screen.getByRole('textbox') as HTMLInputElement
  await fireEvent.input(input, { target: { value: 'New content' } })
  await fireEvent.blur(input)

  expect(mockedAxios.put).toHaveBeenCalledWith('agent/testing/questions/101', {
    content: 'New content'
  })
  await waitFor(() =>
    expect(screen.getByText('New content')).toBeInTheDocument()
  )
  expect(screen.getByText('Untouched')).toBeInTheDocument()
  expect(mockedAxios.get.mock.calls.length).toBe(getCallsBefore)
})

test('a failed question update surfaces an error and keeps the old content', async () => {
  await openSuite([question(101, 'Old content')])
  mockedAxios.put.mockRejectedValue(new Error('500'))

  await fireEvent.click(screen.getByTitle('Rename'))
  const input = screen.getByRole('textbox') as HTMLInputElement
  await fireEvent.input(input, { target: { value: 'New content' } })
  await fireEvent.blur(input)

  await waitFor(() =>
    expect(screen.getByText('Failed to update question')).toBeInTheDocument()
  )
  expect(screen.getByText('Old content')).toBeInTheDocument()
})

test('deleting a question calls the backend and reloads the list', async () => {
  const { state } = await openSuite([
    question(101, 'Doomed'),
    question(102, 'Survivor')
  ])
  mockedAxios.delete.mockImplementation(() => {
    state.questions = [question(102, 'Survivor')]
    return Promise.resolve({ data: {} })
  })

  await fireEvent.click(screen.getAllByTitle('Delete')[0])
  await fireEvent.click(screen.getByText('Yes'))

  expect(mockedAxios.delete).toHaveBeenCalledWith('agent/testing/questions/101')
  await waitFor(() =>
    expect(screen.queryByText('Doomed')).not.toBeInTheDocument()
  )
  expect(screen.getByText('Survivor')).toBeInTheDocument()
})

test('a failed question deletion surfaces an error', async () => {
  await openSuite([question(101, 'Doomed')])
  mockedAxios.delete.mockRejectedValue(new Error('500'))

  await fireEvent.click(screen.getByTitle('Delete'))
  await fireEvent.click(screen.getByText('Yes'))

  await waitFor(() =>
    expect(screen.getByText('Failed to delete question')).toBeInTheDocument()
  )
  expect(screen.getByText('Doomed')).toBeInTheDocument()
})

test('clicking a question dispatches copyQuestion with its content', async () => {
  const onCopy = vi.fn()
  await openSuite([question(101, 'Copy me')], { copyQuestion: onCopy })

  await fireEvent.click(
    screen.getByText('Copy me').closest('.item') as HTMLElement
  )

  expect(onCopy).toHaveBeenCalledTimes(1)
  expect(onCopy.mock.calls[0][0].detail).toEqual({ content: 'Copy me' })
})

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

test('a full run dispatches every question then reports the collected metrics', async () => {
  const onRun = vi.fn()
  const { component } = await openSuite(
    [question(101, 'Q1'), question(102, 'Q2')],
    { runQuestion: onRun }
  )

  const nowSpy = vi.spyOn(Date, 'now')
  nowSpy.mockReturnValue(10_000)

  await fireEvent.click(screen.getByRole('button', { name: /Run Suite/i }))

  expect(onRun).toHaveBeenCalledTimes(1)
  expect(onRun.mock.calls[0][0].detail).toEqual({ content: 'Q1' })
  ;(component as any).handleResponseMetrics({
    usage: { total_tokens: 40 },
    content: 'abcde'
  })
  ;(component as any).handleRunnerNext()

  await waitFor(() =>
    expect(screen.getByText('Running (2/2)')).toBeInTheDocument()
  )
  expect(onRun).toHaveBeenCalledTimes(2)
  expect(onRun.mock.calls[1][0].detail).toEqual({ content: 'Q2' })
  ;(component as any).handleResponseMetrics({
    usage: { total_tokens: 2 },
    content: 'xyz'
  })

  // no more questions -> the run completes
  nowSpy.mockReturnValue(13_500)
  ;(component as any).handleRunnerNext()

  await waitFor(() =>
    expect(screen.getByText('Done in 3.5s')).toBeInTheDocument()
  )
  expect(screen.getByText('42 tokens')).toBeInTheDocument()
  expect(screen.getByText('8 chars')).toBeInTheDocument()
  expect(onRun).toHaveBeenCalledTimes(2)
  // the finished run no longer highlights a question
  expect(screen.getByText('Q2').closest('.item')).not.toHaveClass('active')
})

test('response metrics with missing usage or content are tolerated', async () => {
  const { component } = await openSuite([question(101, 'Q1')])

  await fireEvent.click(screen.getByRole('button', { name: /Run Suite/i }))
  ;(component as any).handleResponseMetrics({ usage: null, content: '' })
  ;(component as any).handleResponseMetrics({ usage: {}, content: 'abc' })
  ;(component as any).handleRunnerNext()

  await waitFor(() => expect(screen.getByText(/Done in/)).toBeInTheDocument())
  expect(screen.getByText('0 tokens')).toBeInTheDocument()
  expect(screen.getByText('3 chars')).toBeInTheDocument()
})

test('response metrics are ignored while the runner is idle', async () => {
  const { component } = await openSuite([question(101, 'Q1')])

  ;(component as any).handleResponseMetrics({
    usage: { total_tokens: 99 },
    content: 'ignored'
  })
  await fireEvent.click(screen.getByRole('button', { name: /Run Suite/i }))
  ;(component as any).handleRunnerNext()

  await waitFor(() => expect(screen.getByText(/Done in/)).toBeInTheDocument())
  expect(screen.getByText('0 tokens')).toBeInTheDocument()
  expect(screen.getByText('0 chars')).toBeInTheDocument()
})

test('handleRunnerNext is a no-op before a run has been started', async () => {
  const onRun = vi.fn()
  const { component } = await openSuite([question(101, 'Q1')], {
    runQuestion: onRun
  })

  ;(component as any).handleRunnerNext()

  expect(onRun).not.toHaveBeenCalled()
  expect(screen.getByRole('button', { name: /Run Suite/i })).toBeInTheDocument()
  expect(screen.queryByText(/Done in/)).not.toBeInTheDocument()
})

test('stopping a run returns to idle without marking it complete', async () => {
  const onRun = vi.fn()
  await openSuite([question(101, 'Q1'), question(102, 'Q2')], {
    runQuestion: onRun
  })

  await fireEvent.click(screen.getByRole('button', { name: /Run Suite/i }))
  await waitFor(() =>
    expect(screen.getByText('Running (1/2)')).toBeInTheDocument()
  )

  await fireEvent.click(screen.getByRole('button', { name: /Stop/i }))

  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: /Run Suite/i })
    ).toBeInTheDocument()
  )
  expect(screen.queryByText(/Done in/)).not.toBeInTheDocument()
  expect(screen.getByText('Q1').closest('.item')).not.toHaveClass('active')
  expect(onRun).toHaveBeenCalledTimes(1)
})

test('an empty suite cannot be run', async () => {
  const onRun = vi.fn()
  await openSuite([], { runQuestion: onRun })

  const runBtn = screen.getByRole('button', { name: /Run Suite/i })
  expect(runBtn).toBeDisabled()

  // even if the click gets through, the guard prevents a run from starting
  await fireEvent.click(runBtn)

  expect(onRun).not.toHaveBeenCalled()
  expect(screen.queryByText(/Running \(/)).not.toBeInTheDocument()
})

// ---------------------------------------------------------------------------
// Import / export
// ---------------------------------------------------------------------------

test('importing a file posts every parsed question and reloads', async () => {
  const { container, state } = await openSuite([])
  mockedParse.mockResolvedValue(['Imported A', 'Imported B'])
  mockedAxios.post.mockImplementation(() => {
    state.questions = [question(301, 'Imported A'), question(302, 'Imported B')]
    return Promise.resolve({ data: {} })
  })

  const fileInput = container.querySelector(
    'input[type="file"]'
  ) as HTMLInputElement
  await fireEvent.change(fileInput, {
    target: { files: [new File(['x'], 'q.xlsx')] }
  })

  await waitFor(() =>
    expect(screen.getByText('Imported A')).toBeInTheDocument()
  )
  expect(screen.getByText('Imported B')).toBeInTheDocument()
  expect(mockedAxios.post).toHaveBeenCalledTimes(2)
  expect(mockedAxios.post.mock.calls[0]).toEqual([
    'agent/testing/suites/1/questions',
    { content: 'Imported A' }
  ])
  expect(mockedAxios.post.mock.calls[1]).toEqual([
    'agent/testing/suites/1/questions',
    { content: 'Imported B' }
  ])
})

test('a failing import POST surfaces the request error message', async () => {
  const { container } = await openSuite([])
  mockedParse.mockResolvedValue(['Imported A'])
  mockedAxios.post.mockRejectedValue(
    new Error('Request failed with status 500')
  )

  const fileInput = container.querySelector(
    'input[type="file"]'
  ) as HTMLInputElement
  await fireEvent.change(fileInput, {
    target: { files: [new File(['x'], 'q.xlsx')] }
  })

  await waitFor(() =>
    expect(
      screen.getByText('Request failed with status 500')
    ).toBeInTheDocument()
  )
})

test('an import failure without a message falls back to a generic error', async () => {
  const { container } = await openSuite([])
  mockedParse.mockResolvedValue(['Imported A'])
  mockedAxios.post.mockRejectedValue({})

  const fileInput = container.querySelector(
    'input[type="file"]'
  ) as HTMLInputElement
  await fireEvent.change(fileInput, {
    target: { files: [new File(['x'], 'q.xlsx')] }
  })

  await waitFor(() =>
    expect(screen.getByText('Failed to import file')).toBeInTheDocument()
  )
})

test('a parser failure reported by the controls is shown as an error', async () => {
  const { container } = await openSuite([])
  mockedParse.mockRejectedValue(new Error('No valid questions found.'))

  const fileInput = container.querySelector(
    'input[type="file"]'
  ) as HTMLInputElement
  await fireEvent.change(fileInput, {
    target: { files: [new File(['x'], 'bad.csv')] }
  })

  await waitFor(() =>
    expect(screen.getByText('No valid questions found.')).toBeInTheDocument()
  )
  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('exporting builds a sheet from the questions and uses a slugged filename', async () => {
  await openSuite([question(101, 'Q one'), question(102, 'Q two')])

  await fireEvent.click(screen.getByRole('button', { name: /Export/i }))

  expect(mockedUtils.json_to_sheet).toHaveBeenCalledWith([
    { questions: 'Q one' },
    { questions: 'Q two' }
  ])
  expect(mockedUtils.book_append_sheet).toHaveBeenCalledWith(
    { kind: 'workbook' },
    { kind: 'worksheet' },
    'Questions'
  )
  expect(mockedWriteFile).toHaveBeenCalledWith(
    { kind: 'workbook' },
    'suite_1_questions.xlsx'
  )
})

test('exporting an empty suite writes nothing', async () => {
  await openSuite([])

  const exportBtn = screen.getByRole('button', { name: /Export/i })
  expect(exportBtn).toBeDisabled()
  await fireEvent.click(exportBtn)

  expect(mockedWriteFile).not.toHaveBeenCalled()
  expect(mockedUtils.json_to_sheet).not.toHaveBeenCalled()
})

test('exporting an unnamed suite falls back to a generic filename', async () => {
  const unnamed: TestSuite = { id: '1', name: '', created_at: 1 }
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/testing/suites')
      return Promise.resolve({ data: [unnamed] })
    return Promise.resolve({ data: [question(101, 'Q one')] })
  })

  const { container } = render(TestingSidebar as Component, {
    props: { isOpen: true }
  })

  await waitFor(() =>
    expect(container.querySelector('.item')).toBeInTheDocument()
  )
  await fireEvent.click(container.querySelector('.item') as HTMLElement)
  await waitFor(() => expect(screen.getByText('Q one')).toBeInTheDocument())

  await fireEvent.click(screen.getByRole('button', { name: /Export/i }))

  expect(mockedWriteFile).toHaveBeenCalledWith(
    { kind: 'workbook' },
    'testing_questions.xlsx'
  )
})
