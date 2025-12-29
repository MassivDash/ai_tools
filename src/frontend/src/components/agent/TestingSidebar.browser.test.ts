/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import TestingSidebar from './TestingSidebar.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance'
import type { Component } from 'svelte'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance', () => ({
  axiosBackendInstance: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
    defaults: { baseURL: 'http://localhost:8000' }
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
  put: ReturnType<typeof vi.fn>
  delete: ReturnType<typeof vi.fn>
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

  const input = screen.getByPlaceholderText('Suite Name')
  await fireEvent.input(input, { target: { value: 'New Suite' } })

  const createBtn = screen.getByText('Create Suite')
  await fireEvent.click(createBtn)

  expect(mockedAxios.post).toHaveBeenCalledWith('agent/testing/suites', {
    name: 'New Suite',
    description: ''
  })

  await waitFor(() => {
    expect(screen.getByText('New Suite')).toBeTruthy()
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

  const suiteItem = screen.getByText('Suite 1').closest('.item')
  expect(suiteItem).toBeTruthy()
  if (suiteItem) await fireEvent.click(suiteItem)

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
