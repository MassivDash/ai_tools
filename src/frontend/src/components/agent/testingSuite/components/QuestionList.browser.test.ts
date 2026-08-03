/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import QuestionList from './QuestionList.svelte'
import type { Component } from 'svelte'
import type { TestQuestion } from '@types'

const makeQuestions = (): TestQuestion[] => [
  { id: 101, suite_id: '1', content: 'First question', created_at: 1 },
  { id: 102, suite_id: '1', content: 'Second question', created_at: 2 }
]

beforeEach(() => {
  vi.spyOn(console, 'log').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('renders each question with a 1-based index and no form by default', () => {
  render(QuestionList as Component, {
    props: { questions: makeQuestions() }
  })

  expect(screen.getByText('First question')).toBeInTheDocument()
  expect(screen.getByText('Second question')).toBeInTheDocument()
  expect(screen.getByText('1.')).toBeInTheDocument()
  expect(screen.getByText('2.')).toBeInTheDocument()
  expect(
    screen.queryByPlaceholderText('Question content...')
  ).not.toBeInTheDocument()
})

test('renders an empty list when no questions are provided at all', () => {
  const { container } = render(QuestionList as Component)

  expect(container.querySelectorAll('.item')).toHaveLength(0)
  expect(container.querySelector('.list')).toBeInTheDocument()
  expect(
    screen.queryByPlaceholderText('Question content...')
  ).not.toBeInTheDocument()
})

test('highlights the current question only while running', async () => {
  const { rerender } = render(QuestionList as Component, {
    props: {
      questions: makeQuestions(),
      currentQuestionIndex: 1,
      running: false
    }
  })

  // index matches but runner is idle -> nothing active
  expect(screen.getByText('Second question').closest('.item')).not.toHaveClass(
    'active'
  )

  await rerender({ running: true })

  expect(screen.getByText('Second question').closest('.item')).toHaveClass(
    'active'
  )
  expect(screen.getByText('First question').closest('.item')).not.toHaveClass(
    'active'
  )
})

test('openAddQuestionForm opens an empty form and Add stays disabled until typing', async () => {
  const { component } = render(QuestionList as Component, {
    props: { questions: makeQuestions() }
  })

  ;(component as any).openAddQuestionForm()

  const textarea = (await screen.findByPlaceholderText(
    'Question content...'
  )) as HTMLTextAreaElement
  expect(textarea.value).toBe('')

  const addBtn = screen.getByRole('button', { name: 'Add Question' })
  expect(addBtn).toBeDisabled()

  await fireEvent.input(textarea, { target: { value: 'New q' } })
  expect(addBtn).not.toBeDisabled()
})

test('saving dispatches save with the typed content and clears the form', async () => {
  const onSave = vi.fn()
  const { component } = render(QuestionList as Component, {
    props: { questions: makeQuestions() },
    events: { save: onSave }
  })

  ;(component as any).openAddQuestionForm()
  const textarea = await screen.findByPlaceholderText('Question content...')
  await fireEvent.input(textarea, { target: { value: 'How tall is it?' } })
  await fireEvent.click(screen.getByRole('button', { name: 'Add Question' }))

  expect(onSave).toHaveBeenCalledTimes(1)
  expect(onSave.mock.calls[0][0].detail).toEqual({
    content: 'How tall is it?'
  })

  await waitFor(() =>
    expect(
      screen.queryByPlaceholderText('Question content...')
    ).not.toBeInTheDocument()
  )

  // re-opening starts from a blank textarea
  ;(component as any).openAddQuestionForm()
  const reopened = (await screen.findByPlaceholderText(
    'Question content...'
  )) as HTMLTextAreaElement
  expect(reopened.value).toBe('')
})

test('whitespace-only content is rejected and the form stays open', async () => {
  const onSave = vi.fn()
  const { component } = render(QuestionList as Component, {
    props: { questions: [] },
    events: { save: onSave }
  })

  ;(component as any).openAddQuestionForm()
  const textarea = await screen.findByPlaceholderText('Question content...')
  await fireEvent.input(textarea, { target: { value: '   ' } })

  // whitespace is truthy so the button is enabled, but the handler bails out
  const addBtn = screen.getByRole('button', { name: 'Add Question' })
  expect(addBtn).not.toBeDisabled()
  await fireEvent.click(addBtn)

  expect(onSave).not.toHaveBeenCalled()
  expect(screen.getByPlaceholderText('Question content...')).toBeInTheDocument()
})

test('cancel closes the form and discards the draft', async () => {
  const onSave = vi.fn()
  const { component } = render(QuestionList as Component, {
    props: { questions: [] },
    events: { save: onSave }
  })

  ;(component as any).openAddQuestionForm()
  const textarea = await screen.findByPlaceholderText('Question content...')
  await fireEvent.input(textarea, { target: { value: 'discard me' } })
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))

  await waitFor(() =>
    expect(
      screen.queryByPlaceholderText('Question content...')
    ).not.toBeInTheDocument()
  )
  expect(onSave).not.toHaveBeenCalled()
  ;(component as any).openAddQuestionForm()
  const reopened = (await screen.findByPlaceholderText(
    'Question content...'
  )) as HTMLTextAreaElement
  expect(reopened.value).toBe('')
})

test('clicking a question dispatches copy with its content', async () => {
  const onCopy = vi.fn()
  render(QuestionList as Component, {
    props: { questions: makeQuestions() },
    events: { copy: onCopy }
  })

  await fireEvent.click(
    screen.getByText('Second question').closest('.item') as HTMLElement
  )

  expect(onCopy).toHaveBeenCalledTimes(1)
  expect(onCopy.mock.calls[0][0].detail).toEqual({
    content: 'Second question'
  })
})

test('renaming a question dispatches update with its id and new content', async () => {
  const onUpdate = vi.fn()
  const onCopy = vi.fn()
  render(QuestionList as Component, {
    props: { questions: makeQuestions() },
    events: { update: onUpdate, copy: onCopy }
  })

  await fireEvent.click(screen.getAllByTitle('Rename')[0])

  const input = screen.getByRole('textbox') as HTMLInputElement
  expect(input.value).toBe('First question')
  await fireEvent.input(input, { target: { value: 'First question v2' } })
  await fireEvent.blur(input)

  expect(onUpdate).toHaveBeenCalledTimes(1)
  expect(onUpdate.mock.calls[0][0].detail).toEqual({
    id: 101,
    content: 'First question v2'
  })
  // editing must not be mistaken for selecting the row
  expect(onCopy).not.toHaveBeenCalled()
})

test('deleting a question requires confirmation and dispatches delete with its id', async () => {
  const onDelete = vi.fn()
  render(QuestionList as Component, {
    props: { questions: makeQuestions() },
    events: { delete: onDelete }
  })

  await fireEvent.click(screen.getAllByTitle('Delete')[1])
  expect(screen.getByText('Delete?')).toBeInTheDocument()
  expect(onDelete).not.toHaveBeenCalled()

  await fireEvent.click(screen.getByText('Yes'))

  expect(onDelete).toHaveBeenCalledTimes(1)
  expect(onDelete.mock.calls[0][0].detail).toEqual({ id: 102 })
})
