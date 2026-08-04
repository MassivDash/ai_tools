/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import CollectionList from './CollectionList.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import { collections, selectedCollection } from '../../../stores/chromadb.ts'
import { get } from 'svelte/store'
import type { ChromaDBCollection } from '@types/chromadb.ts'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    get: vi.fn(),
    delete: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as {
  get: ReturnType<typeof vi.fn>
  delete: ReturnType<typeof vi.fn>
}

// Mock window.confirm
const mockConfirm = vi.fn(() => true)
window.confirm = mockConfirm

beforeEach(() => {
  vi.clearAllMocks()
  collections.set([])
  selectedCollection.set(null)
  // Mock console.error to suppress expected error messages during tests
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

test('renders collection list', () => {
  render(CollectionList)

  expect(screen.getByText('Collections')).toBeTruthy()
})

test('loads collections on mount', async () => {
  const mockCollections: ChromaDBCollection[] = [
    { id: '1', name: 'Collection 1', count: 10 },
    { id: '2', name: 'Collection 2', count: 20 }
  ]

  mockedAxios.get.mockResolvedValueOnce({
    data: {
      success: true,
      data: mockCollections
    }
  })

  render(CollectionList)

  await waitFor(() => {
    expect(mockedAxios.get).toHaveBeenCalledWith('chromadb/collections')
  })

  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
    expect(screen.getByText('Collection 2')).toBeTruthy()
  })
})

test('shows loading state', () => {
  mockedAxios.get.mockImplementation(() => new Promise(() => {})) // Never resolves

  render(CollectionList)

  expect(screen.getByText('Loading collections...')).toBeTruthy()
})

test('shows empty state when no collections', async () => {
  mockedAxios.get.mockResolvedValueOnce({
    data: {
      success: true,
      data: []
    }
  })

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('No collections found')).toBeTruthy()
  })
})

test('shows error when loading fails', async () => {
  mockedAxios.get.mockRejectedValueOnce({
    response: { data: { error: 'Failed to load' } }
  })

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText(/Failed to load/)).toBeTruthy()
  })
})

test('refreshes collections when refresh button is clicked', async () => {
  const mockCollections: ChromaDBCollection[] = [
    { id: '1', name: 'Collection 1', count: 10 }
  ]

  mockedAxios.get.mockResolvedValue({
    data: {
      success: true,
      data: mockCollections
    }
  })

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })

  const refreshButton = screen.getByTitle('Refresh Collections')
  fireEvent.click(refreshButton)

  await waitFor(() => {
    expect(mockedAxios.get).toHaveBeenCalledTimes(2)
  })
})

test('selects collection when card is clicked', async () => {
  const mockCollections: ChromaDBCollection[] = [
    { id: '1', name: 'Collection 1', count: 10 }
  ]

  mockedAxios.get.mockResolvedValueOnce({
    data: {
      success: true,
      data: mockCollections
    }
  })

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })

  const card = screen.getByText('Collection 1').closest('.collection-card')
  fireEvent.click(card!)

  await waitFor(() => {
    expect(get(selectedCollection)?.name).toBe('Collection 1')
  })
})

test('deletes collection when delete is confirmed', async () => {
  const mockCollections: ChromaDBCollection[] = [
    { id: '1', name: 'Collection 1', count: 10 }
  ]

  mockedAxios.get.mockResolvedValueOnce({
    data: {
      success: true,
      data: mockCollections
    }
  })

  mockedAxios.delete.mockResolvedValueOnce({
    data: {
      success: true
    }
  })

  mockedAxios.get.mockResolvedValueOnce({
    data: {
      success: true,
      data: []
    }
  })

  mockConfirm.mockReturnValueOnce(true)

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })

  const deleteButton = screen.getByTitle('Delete collection')
  fireEvent.click(deleteButton)

  await waitFor(() => {
    expect(mockedAxios.delete).toHaveBeenCalledWith(
      'chromadb/collections/Collection 1'
    )
  })
})

const errorText = () => document.querySelector('.error-message')?.textContent

test('shows the API error when the list request is unsuccessful', async () => {
  mockedAxios.get.mockResolvedValueOnce({
    data: { success: false, error: 'chroma is not reachable' }
  })

  render(CollectionList)

  await waitFor(() => {
    expect(errorText()).toBe('chroma is not reachable')
  })
  expect(get(collections)).toEqual([])
})

test('falls back to a generic message when the list request has no error', async () => {
  mockedAxios.get.mockResolvedValueOnce({ data: { success: false } })

  render(CollectionList)

  await waitFor(() => {
    expect(errorText()).toBe('Failed to load collections')
  })
})

test('falls back to the thrown message, then a generic one, on load failure', async () => {
  mockedAxios.get.mockRejectedValueOnce(new Error('socket closed'))

  const { unmount } = render(CollectionList)
  await waitFor(() => {
    expect(errorText()).toBe('socket closed')
  })

  unmount()
  mockedAxios.get.mockRejectedValueOnce({})
  render(CollectionList)
  await waitFor(() => {
    expect(errorText()).toBe('Failed to load collections')
  })
})

test('refreshes the current selection matched by name', async () => {
  selectedCollection.set({ id: 'stale-id', name: 'Collection 1', count: 1 })
  mockedAxios.get.mockResolvedValueOnce({
    data: {
      success: true,
      data: [{ id: 'fresh-id', name: 'Collection 1', count: 99 }]
    }
  })

  render(CollectionList)

  await waitFor(() => {
    expect(get(selectedCollection)).toEqual({
      id: 'fresh-id',
      name: 'Collection 1',
      count: 99
    })
  })
})

test('keeps the current selection when the server no longer returns it', async () => {
  const ghost = { id: 'ghost-id', name: 'Ghost', count: 0 }
  selectedCollection.set(ghost)
  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [{ id: '1', name: 'Collection 1', count: 3 }] }
  })

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })
  expect(get(selectedCollection)).toEqual(ghost)
})

test('does not auto-select a collection without a name', async () => {
  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [{ id: '1', name: '', count: 0 }] }
  })

  render(CollectionList)

  await waitFor(() => {
    expect(get(collections)).toHaveLength(1)
  })
  expect(get(selectedCollection)).toBeNull()
})

test('ignores a click on a card without a name', async () => {
  mockedAxios.get.mockResolvedValueOnce({
    data: {
      success: true,
      data: [
        { id: '1', name: '', count: 0 },
        { id: '2', name: 'Real Collection', count: 5 }
      ]
    }
  })

  const { container } = render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('Real Collection')).toBeTruthy()
  })
  expect(get(selectedCollection)).toBeNull()

  const cards = container.querySelectorAll('.collection-card')
  await fireEvent.click(cards[0])
  expect(get(selectedCollection)).toBeNull()

  await fireEvent.click(cards[1])
  await waitFor(() => {
    expect(get(selectedCollection)?.name).toBe('Real Collection')
  })
})

test('selects the next collection after deleting the selected one', async () => {
  const first = { id: '1', name: 'Collection 1', count: 10 }
  const second = { id: '2', name: 'Collection 2', count: 20 }

  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [first, second] }
  })
  mockedAxios.delete.mockResolvedValueOnce({ data: { success: true } })
  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [second] }
  })
  mockConfirm.mockReturnValueOnce(true)

  render(CollectionList)

  await waitFor(() => {
    expect(get(selectedCollection)?.name).toBe('Collection 1')
  })

  await fireEvent.click(screen.getAllByTitle('Delete collection')[0])

  await waitFor(() => {
    expect(mockedAxios.delete).toHaveBeenCalledWith(
      'chromadb/collections/Collection 1'
    )
  })
  await waitFor(() => {
    expect(get(selectedCollection)).toEqual(second)
  })
  expect(get(collections)).toEqual([second])
})

test('clears the selection when the remaining collection has no name', async () => {
  const first = { id: '1', name: 'Collection 1', count: 10 }
  const nameless = { id: '2', name: '', count: 0 }

  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [first, nameless] }
  })
  mockedAxios.delete.mockResolvedValueOnce({ data: { success: true } })
  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [nameless] }
  })
  mockConfirm.mockReturnValueOnce(true)

  render(CollectionList)

  await waitFor(() => {
    expect(get(selectedCollection)?.name).toBe('Collection 1')
  })

  await fireEvent.click(screen.getAllByTitle('Delete collection')[0])

  await waitFor(() => {
    expect(mockedAxios.delete).toHaveBeenCalled()
  })
  await waitFor(() => {
    expect(get(selectedCollection)).toBeNull()
  })
})

test('keeps the selection when a different collection is deleted', async () => {
  const first = { id: '1', name: 'Collection 1', count: 10 }
  const second = { id: '2', name: 'Collection 2', count: 20 }

  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [first, second] }
  })
  mockedAxios.delete.mockResolvedValueOnce({ data: { success: true } })
  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [first] }
  })
  mockConfirm.mockReturnValueOnce(true)

  render(CollectionList)

  await waitFor(() => {
    expect(get(selectedCollection)?.name).toBe('Collection 1')
  })

  await fireEvent.click(screen.getAllByTitle('Delete collection')[1])

  await waitFor(() => {
    expect(mockedAxios.delete).toHaveBeenCalledWith(
      'chromadb/collections/Collection 2'
    )
  })
  await waitFor(() => {
    expect(get(collections)).toEqual([first])
  })
  expect(get(selectedCollection)).toEqual(first)
})

test('shows the API error when the delete is unsuccessful', async () => {
  const first = { id: '1', name: 'Collection 1', count: 10 }

  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [first] }
  })
  mockedAxios.delete.mockResolvedValueOnce({
    data: { success: false, error: 'collection is locked' }
  })
  mockConfirm.mockReturnValueOnce(true)

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })

  await fireEvent.click(screen.getByTitle('Delete collection'))

  await waitFor(() => {
    expect(errorText()).toBe('collection is locked')
  })
  // the collection is left in place
  expect(get(collections)).toEqual([first])
})

test('falls back to a generic message when the delete response has no error', async () => {
  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [{ id: '1', name: 'Collection 1', count: 1 }] }
  })
  mockedAxios.delete.mockResolvedValueOnce({ data: { success: false } })
  mockConfirm.mockReturnValueOnce(true)

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })

  await fireEvent.click(screen.getByTitle('Delete collection'))

  await waitFor(() => {
    expect(errorText()).toBe('Failed to delete collection')
  })
})

test('shows the backend error payload when the delete request throws', async () => {
  mockedAxios.get.mockResolvedValueOnce({
    data: { success: true, data: [{ id: '1', name: 'Collection 1', count: 1 }] }
  })
  mockedAxios.delete.mockRejectedValueOnce({
    response: { data: { error: 'permission denied' } }
  })
  mockConfirm.mockReturnValueOnce(true)

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })

  await fireEvent.click(screen.getByTitle('Delete collection'))

  await waitFor(() => {
    expect(errorText()).toBe('permission denied')
  })
  expect(console.error).toHaveBeenCalledWith(
    'Error deleting collection:',
    expect.anything()
  )
})

test('falls back to the thrown message, then a generic one, on delete failure', async () => {
  const listOnce = () =>
    mockedAxios.get.mockResolvedValueOnce({
      data: {
        success: true,
        data: [{ id: '1', name: 'Collection 1', count: 1 }]
      }
    })

  listOnce()
  mockedAxios.delete.mockRejectedValueOnce(new Error('gateway timeout'))
  mockConfirm.mockReturnValue(true)

  const { unmount } = render(CollectionList)
  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })
  await fireEvent.click(screen.getByTitle('Delete collection'))
  await waitFor(() => {
    expect(errorText()).toBe('gateway timeout')
  })

  unmount()
  listOnce()
  mockedAxios.delete.mockRejectedValueOnce({})
  render(CollectionList)
  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })
  await fireEvent.click(screen.getByTitle('Delete collection'))
  await waitFor(() => {
    expect(errorText()).toBe('Failed to delete collection')
  })
})

test('does not delete collection when delete is cancelled', async () => {
  const mockCollections: ChromaDBCollection[] = [
    { id: '1', name: 'Collection 1', count: 10 }
  ]

  mockedAxios.get.mockResolvedValueOnce({
    data: {
      success: true,
      data: mockCollections
    }
  })

  mockConfirm.mockReturnValueOnce(false)

  render(CollectionList)

  await waitFor(() => {
    expect(screen.getByText('Collection 1')).toBeTruthy()
  })

  const deleteButton = screen.getByTitle('Delete collection')
  fireEvent.click(deleteButton)

  await waitFor(() => {
    expect(mockedAxios.delete).not.toHaveBeenCalled()
  })
})
