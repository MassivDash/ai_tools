/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import ToolsConfigSection from './ToolsConfigSection.svelte'
import { axiosBackendInstance } from '../../../axiosInstance/axiosBackendInstance.ts'
import type { Component } from 'svelte'

// Mock axiosBackendInstance
vi.mock('../../../axiosInstance/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    get: vi.fn(),
    post: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(console, 'log').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

const defaultProps = {
  enabledTools: [],
  onToggle: vi.fn()
}

const mockTools = [
  {
    name: 'Web Search',
    tool_type: 'web_search',
    description: 'Search the web',
    category: 'search',
    icon: 'magnify'
  },
  {
    name: 'Calculator',
    tool_type: 'calculator',
    description: 'Perform math',
    category: 'utility',
    icon: 'calculator'
  }
]

test('renders loading state initially', async () => {
  mockedAxios.get.mockImplementation(() => new Promise(() => {})) // Never resolves
  render(ToolsConfigSection as Component, { props: defaultProps })
  expect(screen.getByText('Loading tools...')).toBeTruthy()
})

test('renders empty state if no tools', async () => {
  mockedAxios.get.mockResolvedValue({ data: [] })
  render(ToolsConfigSection as Component, { props: defaultProps })
  await waitFor(() => {
    expect(screen.getByText('No tools available')).toBeTruthy()
  })
})

test('renders tools grouped by category', async () => {
  mockedAxios.get.mockResolvedValue({ data: mockTools })
  render(ToolsConfigSection as Component, { props: defaultProps })

  await waitFor(() => {
    expect(screen.getByText('Search')).toBeTruthy()
    expect(screen.getByText('Utility')).toBeTruthy()
    expect(screen.getByText('Web Search')).toBeTruthy()
    expect(screen.getByText('Calculator')).toBeTruthy()
  })
})

test('renders checked state correctly', async () => {
  mockedAxios.get.mockResolvedValue({ data: mockTools })
  render(ToolsConfigSection as Component, {
    props: { ...defaultProps, enabledTools: ['web_search'] }
  })

  await waitFor(() => {
    const searchCheckbox = screen.getByLabelText(
      'Web Search'
    ) as HTMLInputElement
    const calcCheckbox = screen.getByLabelText('Calculator') as HTMLInputElement
    expect(searchCheckbox.checked).toBe(true)
    expect(calcCheckbox.checked).toBe(false)
  })
})

test('logs and shows the empty state when the tool list fails to load', async () => {
  const failure = new Error('tools unavailable')
  mockedAxios.get.mockRejectedValue(failure)
  render(ToolsConfigSection as Component, { props: defaultProps })

  await waitFor(() => {
    expect(screen.getByText('No tools available')).toBeTruthy()
  })
  expect(console.error).toHaveBeenCalledWith(
    'Failed to load available tools:',
    failure
  )
})

test('search filters by display name', async () => {
  mockedAxios.get.mockResolvedValue({ data: mockTools })
  render(ToolsConfigSection as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Web Search'))
  await fireEvent.input(screen.getByPlaceholderText('Search tools...'), {
    target: { value: 'calcul' }
  })

  expect(screen.getByText('Calculator')).toBeTruthy()
  expect(screen.queryByText('Web Search')).toBeNull()
})

test('search also matches the description and the category', async () => {
  mockedAxios.get.mockResolvedValue({ data: mockTools })
  render(ToolsConfigSection as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Web Search'))
  const search = screen.getByPlaceholderText('Search tools...')

  // "Perform math" is only in the Calculator description
  await fireEvent.input(search, { target: { value: 'perform math' } })
  expect(screen.getByText('Calculator')).toBeTruthy()
  expect(screen.queryByText('Web Search')).toBeNull()

  // "search" is the Web Search category (and its name)
  await fireEvent.input(search, { target: { value: 'utility' } })
  expect(screen.getByText('Calculator')).toBeTruthy()
  expect(screen.queryByText('Web Search')).toBeNull()
})

test('shows a no-results message when nothing matches the query', async () => {
  mockedAxios.get.mockResolvedValue({ data: mockTools })
  render(ToolsConfigSection as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Web Search'))
  await fireEvent.input(screen.getByPlaceholderText('Search tools...'), {
    target: { value: 'zzz' }
  })

  expect(screen.getByText('No tools match "zzz"')).toBeTruthy()
  expect(screen.queryByText('Calculator')).toBeNull()

  // The message tracks the query as it keeps changing
  await fireEvent.input(screen.getByPlaceholderText('Search tools...'), {
    target: { value: 'qqq' }
  })
  expect(screen.getByText('No tools match "qqq"')).toBeTruthy()
})

test('tools in the same category are grouped under one header and sorted by name', async () => {
  mockedAxios.get.mockResolvedValue({
    data: [
      ...mockTools,
      {
        name: 'Adder',
        tool_type: 'adder',
        description: 'Add numbers',
        category: 'utility',
        icon: 'plus'
      }
    ]
  })
  render(ToolsConfigSection as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Adder'))
  await fireEvent.click(screen.getByTitle('Group tools by category'))

  const dividers = Array.from(
    document.querySelectorAll('.category-divider')
  ).map((el) => el.textContent?.trim())
  expect(dividers).toEqual(['Search', 'Utility'])

  const rows = Array.from(document.querySelectorAll('.tool-row')).map(
    (el) => el.textContent
  )
  expect(rows[0]).toContain('Web Search')
  // Within the utility category the two tools are sorted by display name
  expect(rows[1]).toContain('Adder')
  expect(rows[2]).toContain('Calculator')
})

test('grouping by category swaps the per-row tags for category headers', async () => {
  mockedAxios.get.mockResolvedValue({ data: mockTools })
  render(ToolsConfigSection as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Web Search'))
  // Ungrouped: each row carries its own category tag
  expect(document.querySelectorAll('.category-tag')).toHaveLength(2)
  expect(document.querySelectorAll('.category-divider')).toHaveLength(0)

  await fireEvent.click(screen.getByTitle('Group tools by category'))

  const dividers = Array.from(
    document.querySelectorAll('.category-divider')
  ).map((el) => el.textContent?.trim())
  expect(dividers).toEqual(['Search', 'Utility'])
  expect(document.querySelectorAll('.category-tag')).toHaveLength(0)
  // Sorted by category, so Web Search (search) comes before Calculator (utility)
  const rows = Array.from(document.querySelectorAll('.tool-row')).map(
    (el) => el.textContent
  )
  expect(rows[0]).toContain('Web Search')
  expect(rows[1]).toContain('Calculator')
})

test('collapses tools that share a tool_type and defaults a missing category', async () => {
  mockedAxios.get.mockResolvedValue({
    data: [
      {
        name: 'Gmail Read',
        tool_type: 'gmail',
        description: 'Read mail',
        category: '',
        icon: 'email'
      },
      {
        name: 'Gmail Send',
        tool_type: 'gmail',
        description: 'Send mail',
        category: '',
        icon: 'email'
      }
    ]
  })
  render(ToolsConfigSection as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Gmail'))
  expect(screen.queryByText('Gmail Read')).toBeNull()
  expect(screen.getByText('Other')).toBeTruthy()
  expect(document.querySelectorAll('.tool-row')).toHaveLength(1)
})

test('calls onToggle when clicked', async () => {
  mockedAxios.get.mockResolvedValue({ data: mockTools })
  const onToggle = vi.fn()
  render(ToolsConfigSection as Component, {
    props: { ...defaultProps, onToggle }
  })

  await waitFor(() => screen.getByText('Web Search'))

  const checkbox = screen.getByLabelText('Web Search')
  await fireEvent.click(checkbox)
  expect(onToggle).toHaveBeenCalledWith('web_search')
})
