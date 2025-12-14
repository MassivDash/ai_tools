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
