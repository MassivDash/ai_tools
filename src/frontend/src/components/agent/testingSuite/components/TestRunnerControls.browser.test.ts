/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import TestRunnerControls from './TestRunnerControls.svelte'
import { parseQuestionsFromFile } from '../../utils/testingUtils'
import type { Component } from 'svelte'

vi.mock('../../utils/testingUtils', () => ({
  parseQuestionsFromFile: vi.fn()
}))

const mockedParse = parseQuestionsFromFile as unknown as ReturnType<
  typeof vi.fn
>

const getFileInput = (container: HTMLElement) =>
  container.querySelector('input[type="file"]') as HTMLInputElement

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(console, 'log').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('idle state shows Run Suite plus the import/export controls', () => {
  render(TestRunnerControls as Component, {
    props: { questionsCount: 3 }
  })

  const runBtn = screen.getByRole('button', { name: /Run Suite/i })
  expect(runBtn).toBeInTheDocument()
  expect(runBtn).not.toBeDisabled()
  expect(screen.getByRole('button', { name: /Import/i })).toBeInTheDocument()
  expect(screen.getByRole('button', { name: /Export/i })).not.toBeDisabled()
  expect(
    screen.queryByRole('button', { name: /Stop/i })
  ).not.toBeInTheDocument()
  expect(screen.queryByText(/Done in/)).not.toBeInTheDocument()
})

test('clicking Run Suite dispatches start', async () => {
  const onStart = vi.fn()
  render(TestRunnerControls as Component, {
    props: { questionsCount: 2 },
    events: { start: onStart }
  })

  await fireEvent.click(screen.getByRole('button', { name: /Run Suite/i }))

  expect(onStart).toHaveBeenCalledTimes(1)
})

test('an empty suite disables Run Suite and Export but keeps Import usable', async () => {
  const { rerender } = render(TestRunnerControls as Component, {
    props: { questionsCount: 0 }
  })

  expect(screen.getByRole('button', { name: /Run Suite/i })).toBeDisabled()
  expect(screen.getByRole('button', { name: /Export/i })).toBeDisabled()
  expect(screen.getByRole('button', { name: /Import/i })).not.toBeDisabled()

  // adding a question re-enables them
  await rerender({ questionsCount: 1 })

  expect(screen.getByRole('button', { name: /Run Suite/i })).not.toBeDisabled()
  expect(screen.getByRole('button', { name: /Export/i })).not.toBeDisabled()
})

test('running state shows progress as a 1-based counter and hides import/export', async () => {
  const onStop = vi.fn()
  render(TestRunnerControls as Component, {
    props: {
      running: true,
      runStatus: 'running',
      questionsCount: 5,
      currentQuestionIndex: 2
    },
    events: { stop: onStop }
  })

  expect(screen.getByText('Running (3/5)')).toBeInTheDocument()
  expect(
    screen.queryByRole('button', { name: /Run Suite/i })
  ).not.toBeInTheDocument()
  expect(
    screen.queryByRole('button', { name: /Import/i })
  ).not.toBeInTheDocument()
  expect(
    screen.queryByRole('button', { name: /Export/i })
  ).not.toBeInTheDocument()

  await fireEvent.click(screen.getByRole('button', { name: /Stop/i }))
  expect(onStop).toHaveBeenCalledTimes(1)
})

test('progress counter defaults to the first question when no index is given', () => {
  render(TestRunnerControls as Component, {
    props: { running: true, runStatus: 'running', questionsCount: 3 }
  })

  expect(screen.getByText('Running (1/3)')).toBeInTheDocument()
})

test('progress counter tracks currentQuestionIndex updates', async () => {
  const { rerender } = render(TestRunnerControls as Component, {
    props: {
      running: true,
      runStatus: 'running',
      questionsCount: 4,
      currentQuestionIndex: 0
    }
  })

  expect(screen.getByText('Running (1/4)')).toBeInTheDocument()

  await rerender({ currentQuestionIndex: 3 })

  expect(screen.getByText('Running (4/4)')).toBeInTheDocument()
})

test('completed state reports elapsed seconds, tokens and chars', () => {
  render(TestRunnerControls as Component, {
    props: {
      running: false,
      runStatus: 'completed',
      questionsCount: 2,
      startTime: 1_000,
      endTime: 3_500,
      totalTokens: 1234,
      totalChars: 5678
    }
  })

  expect(screen.getByText('Done in 2.5s')).toBeInTheDocument()
  expect(screen.getByText('1234 tokens')).toBeInTheDocument()
  expect(screen.getByText('5678 chars')).toBeInTheDocument()
})

test('completed state falls back to 0 when timings are missing', () => {
  render(TestRunnerControls as Component, {
    props: {
      runStatus: 'completed',
      questionsCount: 1,
      startTime: null,
      endTime: null
    }
  })

  expect(screen.getByText('Done in 0s')).toBeInTheDocument()
  expect(screen.getByText('0 tokens')).toBeInTheDocument()
  expect(screen.getByText('0 chars')).toBeInTheDocument()
})

test('clicking Export dispatches export', async () => {
  const onExport = vi.fn()
  render(TestRunnerControls as Component, {
    props: { questionsCount: 1 },
    events: { export: onExport }
  })

  await fireEvent.click(screen.getByRole('button', { name: /Export/i }))

  expect(onExport).toHaveBeenCalledTimes(1)
})

test('clicking Import forwards the click to the hidden file input', async () => {
  const { container } = render(TestRunnerControls as Component, {
    props: { questionsCount: 1 }
  })

  const input = getFileInput(container)
  expect(input).toHaveAttribute('accept', '.xlsx, .xls, .csv')
  const clickSpy = vi.spyOn(input, 'click').mockImplementation(() => {})

  await fireEvent.click(screen.getByRole('button', { name: /Import/i }))

  expect(clickSpy).toHaveBeenCalledTimes(1)
})

test('picking a file dispatches import with the parsed questions and resets the input', async () => {
  mockedParse.mockResolvedValue(['Q one', 'Q two'])
  const onImport = vi.fn()
  const onError = vi.fn()
  const { container } = render(TestRunnerControls as Component, {
    props: { questionsCount: 0 },
    events: { import: onImport, error: onError }
  })

  const input = getFileInput(container)
  const file = new File(['binary'], 'questions.xlsx', {
    type: 'application/vnd.ms-excel'
  })
  await fireEvent.change(input, { target: { files: [file] } })

  await waitFor(() => expect(onImport).toHaveBeenCalledTimes(1))
  expect(mockedParse).toHaveBeenCalledWith(file)
  expect(onImport.mock.calls[0][0].detail).toEqual({
    questions: ['Q one', 'Q two']
  })
  expect(onError).not.toHaveBeenCalled()
  expect(input.value).toBe('')
})

test('a parse failure dispatches error with the parser message', async () => {
  mockedParse.mockRejectedValue(new Error('No valid questions found.'))
  const onImport = vi.fn()
  const onError = vi.fn()
  const { container } = render(TestRunnerControls as Component, {
    props: { questionsCount: 0 },
    events: { import: onImport, error: onError }
  })

  await fireEvent.change(getFileInput(container), {
    target: { files: [new File(['x'], 'bad.csv')] }
  })

  await waitFor(() => expect(onError).toHaveBeenCalledTimes(1))
  expect(onError.mock.calls[0][0].detail).toEqual({
    message: 'No valid questions found.'
  })
  expect(onImport).not.toHaveBeenCalled()
})

test('a parse failure without a message falls back to a generic error', async () => {
  mockedParse.mockRejectedValue({})
  const onError = vi.fn()
  const { container } = render(TestRunnerControls as Component, {
    props: { questionsCount: 0 },
    events: { error: onError }
  })

  await fireEvent.change(getFileInput(container), {
    target: { files: [new File(['x'], 'bad.csv')] }
  })

  await waitFor(() => expect(onError).toHaveBeenCalledTimes(1))
  expect(onError.mock.calls[0][0].detail).toEqual({
    message: 'Failed to import file'
  })
})

test('a change event with no selected file is ignored', async () => {
  const onImport = vi.fn()
  const onError = vi.fn()
  const { container } = render(TestRunnerControls as Component, {
    props: { questionsCount: 0 },
    events: { import: onImport, error: onError }
  })

  await fireEvent.change(getFileInput(container), { target: { files: [] } })

  expect(mockedParse).not.toHaveBeenCalled()
  expect(onImport).not.toHaveBeenCalled()
  expect(onError).not.toHaveBeenCalled()
})
