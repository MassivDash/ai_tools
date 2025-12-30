/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import ChromaDBManager from './ChromaDBManager.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import { collections, selectedCollection } from '../../stores/chromadb'
import type { ChromaDBHealthResponse } from '../../types/chromadb.ts'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    get: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  collections.set([])
  selectedCollection.set(null)
  // Mock console.error to suppress expected error messages during tests
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

test('renders ChromaDB manager', async () => {
  // Mock API calls
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'chromadb/health') {
      return Promise.resolve({
        data: {
          success: true,
          data: {
            status: 'healthy',
            version: '1.0.0',
            chromadb: { connected: true }
          }
        }
      })
    }
    if (url === 'chromadb/collections') {
      return Promise.resolve({
        data: {
          success: true,
          data: []
        }
      })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(ChromaDBManager)

  await waitFor(
    () => {
      // Check for the status text specifically, not the config panel header
      expect(screen.getByText(/ChromaDB (Connected|Disconnected)/)).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('checks health on mount', async () => {
  const mockHealth: ChromaDBHealthResponse = {
    status: 'healthy',
    version: '1.0.0',
    chromadb: { connected: true }
  }

  // Mock both health check and collections API calls (CollectionList also calls on mount)
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'chromadb/health') {
      return Promise.resolve({
        data: {
          success: true,
          data: mockHealth
        }
      })
    }
    if (url === 'chromadb/collections') {
      return Promise.resolve({
        data: {
          success: true,
          data: []
        }
      })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(ChromaDBManager)

  await waitFor(
    () => {
      expect(mockedAxios.get).toHaveBeenCalledWith('chromadb/health')
    },
    { timeout: 2000 }
  )

  await waitFor(
    () => {
      expect(screen.getByText('ChromaDB Connected')).toBeTruthy()
      expect(screen.getByText('v1.0.0')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('shows disconnected status when health check fails', async () => {
  // Mock collections API call (CollectionList calls on mount)
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'chromadb/health') {
      return Promise.reject(new Error('Connection failed'))
    }
    if (url === 'chromadb/collections') {
      return Promise.resolve({
        data: {
          success: true,
          data: []
        }
      })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(ChromaDBManager)

  await waitFor(
    () => {
      expect(screen.getByText('ChromaDB Disconnected')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('refreshes health when refresh button is clicked', async () => {
  const mockHealth: ChromaDBHealthResponse = {
    status: 'healthy',
    version: '1.0.0',
    chromadb: { connected: true }
  }

  // Mock both health check and collections API calls
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'chromadb/health') {
      return Promise.resolve({
        data: {
          success: true,
          data: mockHealth
        }
      })
    }
    if (url === 'chromadb/collections') {
      return Promise.resolve({
        data: {
          success: true,
          data: []
        }
      })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(ChromaDBManager)

  await waitFor(
    () => {
      expect(screen.getByText('ChromaDB Connected')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // Get the initial health call count
  const initialHealthCalls = mockedAxios.get.mock.calls.filter(
    (call) => call[0] === 'chromadb/health'
  ).length

  const refreshButton = screen.getByTitle('Refresh Health Status')
  fireEvent.click(refreshButton)

  await waitFor(
    () => {
      // Should have one more health call than initially
      const healthCalls = mockedAxios.get.mock.calls.filter(
        (call) => call[0] === 'chromadb/health'
      )
      expect(healthCalls.length).toBeGreaterThan(initialHealthCalls)
    },
    { timeout: 2000 }
  )
})

test('shows no selection message when no collections', async () => {
  collections.set([])

  // Mock API calls
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'chromadb/health') {
      return Promise.resolve({
        data: {
          success: true,
          data: {
            status: 'healthy',
            version: '1.0.0',
            chromadb: { connected: true }
          }
        }
      })
    }
    if (url === 'chromadb/collections') {
      return Promise.resolve({
        data: {
          success: true,
          data: []
        }
      })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(ChromaDBManager)

  await waitFor(
    () => {
      expect(
        screen.getByText('No collections, add collection to start')
      ).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('shows no selection message when collection not selected', async () => {
  collections.set([{ id: '1', name: 'Collection 1', count: 10 }])
  selectedCollection.set(null)

  // Mock API calls
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'chromadb/health') {
      return Promise.resolve({
        data: {
          success: true,
          data: {
            status: 'healthy',
            version: '1.0.0',
            chromadb: { connected: true }
          }
        }
      })
    }
    if (url === 'chromadb/collections') {
      return Promise.resolve({
        data: {
          success: true,
          data: [{ id: '1', name: 'Collection 1', count: 10 }]
        }
      })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(ChromaDBManager)

  await waitFor(
    () => {
      expect(screen.getByText(/Select a collection from the left/)).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('shows selected collection when one is selected', async () => {
  const collection = { id: '1', name: 'Collection 1', count: 10 }
  collections.set([collection])
  selectedCollection.set(collection)

  // Mock API calls
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'chromadb/health') {
      return Promise.resolve({
        data: {
          success: true,
          data: {
            status: 'healthy',
            version: '1.0.0',
            chromadb: { connected: true }
          }
        }
      })
    }
    if (url === 'chromadb/collections') {
      return Promise.resolve({
        data: {
          success: true,
          data: [collection]
        }
      })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(ChromaDBManager)

  await waitFor(
    () => {
      expect(screen.getByText('Collection: Collection 1')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})
