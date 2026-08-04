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

const setViewportWidth = (width: number) => {
  Object.defineProperty(window, 'innerWidth', {
    value: width,
    configurable: true,
    writable: true
  })
}

const conversation = (id: string, title: string) => ({
  id,
  title,
  model: 'llama2',
  created_at: 0
})

test('logs and shows the empty list when loading conversations fails', async () => {
  const failure = new Error('boom')
  mockedAxios.get.mockRejectedValue(failure)

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: undefined }
  })

  await waitFor(() => {
    expect(screen.getByText('No history yet')).toBeTruthy()
  })
  expect(console.error).toHaveBeenCalledWith(
    'Failed to load conversations:',
    failure
  )
})

test('does not load conversations while closed, and reloads when shouldRefresh flips on', async () => {
  mockedAxios.get.mockResolvedValue({ data: [] })

  const { rerender } = render(HistorySidebar as Component, {
    props: { isOpen: false, currentConversationId: undefined }
  })

  // onMount still performs the initial load, but the isOpen/shouldRefresh
  // reactive blocks must not add a second request while closed.
  await waitFor(() => expect(mockedAxios.get).toHaveBeenCalledTimes(1))

  await rerender({
    isOpen: false,
    currentConversationId: undefined,
    shouldRefresh: true
  })

  await waitFor(() => expect(mockedAxios.get).toHaveBeenCalledTimes(2))
})

test('an untitled conversation falls back to a placeholder title', async () => {
  mockedAxios.get.mockResolvedValue({ data: [conversation('1', '')] })

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: '1' }
  })

  await waitFor(() => {
    expect(screen.getByText('New Conversation')).toBeTruthy()
  })
})

test('closing the sidebar calls onClose', async () => {
  mockedAxios.get.mockResolvedValue({ data: [] })
  const onClose = vi.fn()

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: undefined, onClose }
  })

  await fireEvent.click(screen.getByTitle('Close History'))

  expect(onClose).toHaveBeenCalledTimes(1)
})

test('selecting a conversation on a narrow viewport also closes the sidebar', async () => {
  setViewportWidth(500)
  mockedAxios.get.mockResolvedValue({ data: [conversation('1', 'Chat 1')] })
  const onSelect = vi.fn()
  const onClose = vi.fn()

  render(HistorySidebar as Component, {
    props: {
      isOpen: true,
      currentConversationId: undefined,
      onSelect,
      onClose
    }
  })

  await waitFor(() => screen.getByText('Chat 1'))
  await fireEvent.click(screen.getByText('Chat 1'))

  expect(onSelect).toHaveBeenCalledWith('1')
  expect(onClose).toHaveBeenCalledTimes(1)
  setViewportWidth(1024)
})

test('starting a new chat on a narrow viewport also closes the sidebar', async () => {
  setViewportWidth(500)
  mockedAxios.get.mockResolvedValue({ data: [] })
  const onNew = vi.fn()
  const onClose = vi.fn()

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: undefined, onNew, onClose }
  })

  await fireEvent.click(screen.getByTitle('New Chat'))

  expect(onNew).toHaveBeenCalledTimes(1)
  expect(onClose).toHaveBeenCalledTimes(1)
  setViewportWidth(1024)
})

test('select, new chat and close are all safe without callbacks', async () => {
  setViewportWidth(500)
  mockedAxios.get.mockResolvedValue({ data: [conversation('1', 'Chat 1')] })

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: undefined }
  })

  await waitFor(() => screen.getByText('Chat 1'))
  await fireEvent.click(screen.getByText('Chat 1'))
  await fireEvent.click(screen.getByTitle('New Chat'))
  await fireEvent.click(screen.getByTitle('Close History'))

  // Nothing to observe other than the component surviving the interactions
  expect(screen.getByText('Chat 1')).toBeTruthy()
  setViewportWidth(1024)
})

test('renaming a conversation patches the backend and updates only that row', async () => {
  mockedAxios.get.mockResolvedValue({
    data: [conversation('1', 'Chat 1'), conversation('2', 'Chat 2')]
  })
  mockedAxios.patch.mockResolvedValue({ data: { success: true } })

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: undefined }
  })

  await waitFor(() => screen.getByText('Chat 1'))
  await fireEvent.click(screen.getAllByTitle('Rename')[0])

  const input = screen.getByRole('textbox') as HTMLInputElement
  await fireEvent.input(input, { target: { value: 'Renamed' } })
  await fireEvent.blur(input)

  await waitFor(() => {
    expect(mockedAxios.patch).toHaveBeenCalledWith('agent/conversations/1', {
      title: 'Renamed'
    })
  })
  await waitFor(() => expect(screen.getByText('Renamed')).toBeTruthy())
  expect(screen.getByText('Chat 2')).toBeTruthy()
  expect(screen.queryByText('Chat 1')).toBeNull()
})

test('a failed rename is logged and leaves the title unchanged', async () => {
  mockedAxios.get.mockResolvedValue({ data: [conversation('1', 'Chat 1')] })
  const failure = new Error('patch failed')
  mockedAxios.patch.mockRejectedValue(failure)

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: undefined }
  })

  await waitFor(() => screen.getByText('Chat 1'))
  await fireEvent.click(screen.getByTitle('Rename'))

  const input = screen.getByRole('textbox') as HTMLInputElement
  await fireEvent.input(input, { target: { value: 'Renamed' } })
  await fireEvent.blur(input)

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to update title:',
      failure
    )
  })
  expect(screen.getByText('Chat 1')).toBeTruthy()
  expect(screen.queryByText('Renamed')).toBeNull()
})

test('deleting a conversation removes it from the list after confirmation', async () => {
  mockedAxios.get.mockResolvedValue({
    data: [conversation('1', 'Chat 1'), conversation('2', 'Chat 2')]
  })
  mockedAxios.delete.mockResolvedValue({ data: { success: true } })
  const onNew = vi.fn()

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: '2', onNew }
  })

  await waitFor(() => screen.getByText('Chat 1'))
  await fireEvent.click(screen.getAllByTitle('Delete')[0])
  await fireEvent.click(screen.getByText('Yes'))

  await waitFor(() => {
    expect(mockedAxios.delete).toHaveBeenCalledWith('agent/conversations/1')
  })
  await waitFor(() => expect(screen.queryByText('Chat 1')).toBeNull())
  expect(screen.getByText('Chat 2')).toBeTruthy()
  // A different conversation was open, so no new chat should be started
  expect(onNew).not.toHaveBeenCalled()
})

test('deleting the currently open conversation starts a new chat', async () => {
  setViewportWidth(1024)
  mockedAxios.get.mockResolvedValue({ data: [conversation('1', 'Chat 1')] })
  mockedAxios.delete.mockResolvedValue({ data: { success: true } })
  const onNew = vi.fn()
  const onClose = vi.fn()

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: '1', onNew, onClose }
  })

  await waitFor(() => screen.getByText('Chat 1'))
  await fireEvent.click(screen.getByTitle('Delete'))
  await fireEvent.click(screen.getByText('Yes'))

  await waitFor(() => expect(onNew).toHaveBeenCalledTimes(1))
  // Wide viewport, so the sidebar stays open
  expect(onClose).not.toHaveBeenCalled()
})

test('a failed delete is logged and keeps the conversation in the list', async () => {
  mockedAxios.get.mockResolvedValue({ data: [conversation('1', 'Chat 1')] })
  const failure = new Error('delete failed')
  mockedAxios.delete.mockRejectedValue(failure)

  render(HistorySidebar as Component, {
    props: { isOpen: true, currentConversationId: '1' }
  })

  await waitFor(() => screen.getByText('Chat 1'))
  await fireEvent.click(screen.getByTitle('Delete'))
  await fireEvent.click(screen.getByText('Yes'))

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to delete conversation:',
      failure
    )
  })
  expect(screen.getByText('Chat 1')).toBeTruthy()
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
