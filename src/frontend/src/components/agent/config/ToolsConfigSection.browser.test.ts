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
        const searchCheckbox = screen.getByLabelText('Web Search') as HTMLInputElement
        const calcCheckbox = screen.getByLabelText('Calculator') as HTMLInputElement
        expect(searchCheckbox.checked).toBe(true)
        expect(calcCheckbox.checked).toBe(false)
    })
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
