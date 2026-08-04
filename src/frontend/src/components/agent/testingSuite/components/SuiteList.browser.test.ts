/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import SuiteList from './SuiteList.svelte'
import type { Component } from 'svelte'
import type { TestSuite } from '@types'

const suites: TestSuite[] = [
  { id: 'a', name: 'Alpha', description: 'First suite', created_at: 1 },
  { id: 'b', name: 'Beta', created_at: 2 }
]

beforeEach(() => {
  vi.spyOn(console, 'log').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('renders the description only when the suite has one, and marks the selected suite', () => {
  const { container } = render(SuiteList as Component, {
    props: { suites, selectedSuiteId: 'b' }
  })

  expect(screen.getByText('First suite')).toBeInTheDocument()
  expect(container.querySelectorAll('.desc')).toHaveLength(1)
  expect(screen.getByText('Alpha').closest('.item')).not.toHaveClass('active')
  expect(screen.getByText('Beta').closest('.item')).toHaveClass('active')
})

test('renders an empty list when no suites are provided at all', () => {
  const { container } = render(SuiteList as Component)

  expect(container.querySelectorAll('.item')).toHaveLength(0)
  expect(container.querySelector('.list')).toBeInTheDocument()
  expect(screen.queryByPlaceholderText('Suite Name')).not.toBeInTheDocument()
})

test('clicking a suite dispatches select with the whole suite object', async () => {
  const onSelect = vi.fn()
  render(SuiteList as Component, {
    props: { suites },
    events: { select: onSelect }
  })

  await fireEvent.click(
    screen.getByText('Alpha').closest('.item') as HTMLElement
  )

  expect(onSelect).toHaveBeenCalledTimes(1)
  expect(onSelect.mock.calls[0][0].detail).toEqual({ suite: suites[0] })
})

test('openNewSuiteForm creates a suite with name and description', async () => {
  const onCreate = vi.fn()
  const { component } = render(SuiteList as Component, {
    props: { suites: [] },
    events: { create: onCreate }
  })

  ;(component as any).openNewSuiteForm()

  const nameInput = await screen.findByPlaceholderText('Suite Name')
  expect(screen.getByRole('button', { name: 'Create Suite' })).toBeDisabled()

  await fireEvent.input(nameInput, { target: { value: 'Gamma' } })
  await fireEvent.input(screen.getByPlaceholderText('Description'), {
    target: { value: 'Third suite' }
  })
  await fireEvent.click(screen.getByRole('button', { name: 'Create Suite' }))

  expect(onCreate).toHaveBeenCalledTimes(1)
  expect(onCreate.mock.calls[0][0].detail).toEqual({
    name: 'Gamma',
    description: 'Third suite'
  })
  await waitFor(() =>
    expect(screen.queryByPlaceholderText('Suite Name')).not.toBeInTheDocument()
  )
})

test('a whitespace-only name does not create a suite', async () => {
  const onCreate = vi.fn()
  const { component } = render(SuiteList as Component, {
    props: { suites: [] },
    events: { create: onCreate }
  })

  ;(component as any).openNewSuiteForm()
  const nameInput = await screen.findByPlaceholderText('Suite Name')
  await fireEvent.input(nameInput, { target: { value: '  ' } })
  await fireEvent.click(screen.getByRole('button', { name: 'Create Suite' }))

  expect(onCreate).not.toHaveBeenCalled()
  expect(screen.getByPlaceholderText('Suite Name')).toBeInTheDocument()
})

test('cancel closes the form and clears the draft', async () => {
  const onCreate = vi.fn()
  const { component } = render(SuiteList as Component, {
    props: { suites: [] },
    events: { create: onCreate }
  })

  ;(component as any).openNewSuiteForm()
  await fireEvent.input(await screen.findByPlaceholderText('Suite Name'), {
    target: { value: 'Discarded' }
  })
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))

  await waitFor(() =>
    expect(screen.queryByPlaceholderText('Suite Name')).not.toBeInTheDocument()
  )
  ;(component as any).openNewSuiteForm()
  const reopened = (await screen.findByPlaceholderText(
    'Suite Name'
  )) as HTMLInputElement
  expect(reopened.value).toBe('')
  expect(onCreate).not.toHaveBeenCalled()
})

test('renaming keeps the existing description, defaulting to an empty string', async () => {
  const onSave = vi.fn()
  render(SuiteList as Component, {
    props: { suites },
    events: { save: onSave }
  })

  // suite with a description
  await fireEvent.click(screen.getAllByTitle('Rename')[0])
  let input = screen.getByRole('textbox') as HTMLInputElement
  await fireEvent.input(input, { target: { value: 'Alpha renamed' } })
  await fireEvent.blur(input)

  expect(onSave.mock.calls[0][0].detail).toEqual({
    id: 'a',
    name: 'Alpha renamed',
    description: 'First suite'
  })

  // suite without a description
  await fireEvent.click(screen.getAllByTitle('Rename')[1])
  input = screen.getByRole('textbox') as HTMLInputElement
  await fireEvent.input(input, { target: { value: 'Beta renamed' } })
  await fireEvent.blur(input)

  expect(onSave.mock.calls[1][0].detail).toEqual({
    id: 'b',
    name: 'Beta renamed',
    description: ''
  })
})

test('deleting a suite requires confirmation and dispatches its id', async () => {
  const onDelete = vi.fn()
  render(SuiteList as Component, {
    props: { suites },
    events: { delete: onDelete }
  })

  await fireEvent.click(screen.getAllByTitle('Delete')[1])
  await fireEvent.click(screen.getByText('No'))
  expect(onDelete).not.toHaveBeenCalled()

  await fireEvent.click(screen.getAllByTitle('Delete')[1])
  await fireEvent.click(screen.getByText('Yes'))

  expect(onDelete).toHaveBeenCalledTimes(1)
  expect(onDelete.mock.calls[0][0].detail).toEqual({ id: 'b' })
})
