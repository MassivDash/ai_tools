/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import HistorySidebar from './HistorySidebar.svelte'
import { axiosBackendInstance } from '../../../axiosInstance/axiosBackendInstance'
import type { Component } from 'svelte'

// Mock axiosBackendInstance
vi.mock('../../../axiosInstance/axiosBackendInstance', () => ({
  axiosBackendInstance: {
    get: vi.fn(),
    post: vi.fn(),
    patch: vi.fn(),
    delete: vi.fn(),
    defaults: { baseURL: 'http://localhost:8000' }
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
  patch: ReturnType<typeof vi.fn>
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

test('loads conversations on open', async () => {
  const conversations = [
    { id: '1', title: 'Chat 1', model: 'llama2', created_at: Date.now() },
    { id: '2', title: 'Chat 2', model: 'gpt4', created_at: Date.now() }
  ]
  mockedAxios.get.mockResolvedValue({ data: conversations })

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: undefined }
  })

  await waitFor(() => {
    expect(screen.getByText('Chat 1')).toBeTruthy()
    expect(screen.getByText('Chat 2')).toBeTruthy()
  })

  expect(mockedAxios.get).toHaveBeenCalledWith('agent/conversations')
})

test('shows empty state when no conversations', async () => {
    mockedAxios.get.mockResolvedValue({ data: [] })
  
    render(HistorySidebar as Component, {
      props: { isOpen: true, currentConversationId: undefined }
    })
  
    await waitFor(() => {
      expect(screen.getByText('No history yet')).toBeTruthy()
    })
})

test('clicking new chat dispatches event', async () => {
    mockedAxios.get.mockResolvedValue({ data: [] })
    const newChatSpy = vi.fn()
    
    render(HistorySidebar as Component, {
        props: { 
            isOpen: true, 
            currentConversationId: undefined,
            onNew: newChatSpy
        }
    })

    const newBtn = screen.getByTitle('New Chat')
    await fireEvent.click(newBtn)

    expect(newChatSpy).toHaveBeenCalled()
})

test('selecting a conversation dispatches event', async () => {
    const conversations = [
        { id: '1', title: 'Chat 1', model: 'llama2', created_at: Date.now() }
    ]
    mockedAxios.get.mockResolvedValue({ data: conversations })
    
    const selectSpy = vi.fn()

    render(HistorySidebar as Component, {
        props: { 
            isOpen: true, 
            currentConversationId: undefined,
            onSelect: selectSpy
        }
    })

    await waitFor(() => expect(screen.getByText('Chat 1')).toBeTruthy())

    // Click the item (EditableListItem renders title in a span)
    const item = screen.getByText('Chat 1')
    await fireEvent.click(item)

    expect(selectSpy).toHaveBeenCalledWith('1')
})

