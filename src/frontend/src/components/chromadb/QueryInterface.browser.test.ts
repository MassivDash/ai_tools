/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import QueryInterface from './QueryInterface.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import type { QueryResponse } from '@types/chromadb.ts'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    post: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  post: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
})

test('renders query interface', () => {
  render(QueryInterface, {
    props: { selectedCollection: 'test-collection' }
  })

  expect(screen.getByText('Search Collection')).toBeTruthy()
  expect(screen.getByPlaceholderText('Enter your search query...')).toBeTruthy()
})

test('shows warning when no collection is selected', () => {
  render(QueryInterface, {
    props: { selectedCollection: null }
  })

  expect(
    screen.getByText('⚠️ Please select a collection first to search')
  ).toBeTruthy()
})

test('performs query successfully', async () => {
  const mockResponse: QueryResponse = {
    ids: [['id1', 'id2']],
    distances: [[0.1, 0.2]],
    documents: [['doc1', 'doc2']],
    metadatas: [[{ source: 'test' }, { source: 'test2' }]]
  }

  mockedAxios.post.mockResolvedValueOnce({
    data: {
      success: true,
      data: mockResponse
    }
  })

  render(QueryInterface, {
    props: { selectedCollection: 'test-collection' }
  })

  const queryInput = screen.getByPlaceholderText('Enter your search query...')
  fireEvent.input(queryInput, { target: { value: 'test query' } })

  const searchButton = screen.getByText('Search')
  fireEvent.click(searchButton)

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'chromadb/query',
      expect.objectContaining({
        collection: 'test-collection',
        query_texts: ['test query'],
        n_results: 10
      })
    )
  })

  await waitFor(() => {
    expect(screen.getByText('Results (2)')).toBeTruthy()
    expect(screen.getByText('doc1')).toBeTruthy()
    expect(screen.getByText('doc2')).toBeTruthy()
  })
})

test('shows error when query fails', async () => {
  mockedAxios.post.mockResolvedValueOnce({
    data: {
      success: false,
      error: 'Query failed'
    }
  })

  render(QueryInterface, {
    props: { selectedCollection: 'test-collection' }
  })

  const queryInput = screen.getByPlaceholderText('Enter your search query...')
  fireEvent.input(queryInput, { target: { value: 'test query' } })

  const searchButton = screen.getByText('Search')
  fireEvent.click(searchButton)

  await waitFor(() => {
    expect(screen.getByText(/Query failed/)).toBeTruthy()
  })
})

test('disables search button when query is empty', () => {
  render(QueryInterface, {
    props: { selectedCollection: 'test-collection' }
  })

  const searchButton = screen.getByText('Search')
  expect(searchButton).toBeDisabled()
})

test('allows changing number of results', () => {
  render(QueryInterface, {
    props: { selectedCollection: 'test-collection' }
  })

  const nResultsInput = screen.getByLabelText(
    'Number of Results'
  ) as HTMLInputElement
  fireEvent.input(nResultsInput, { target: { value: '20' } })

  expect(nResultsInput.value).toBe('20')
})

test('performs query on Enter key press', async () => {
  const mockResponse: QueryResponse = {
    ids: [['id1']],
    distances: [[0.1]],
    documents: [['doc1']],
    metadatas: [[{ source: 'test' }]]
  }

  mockedAxios.post.mockResolvedValueOnce({
    data: {
      success: true,
      data: mockResponse
    }
  })

  render(QueryInterface, {
    props: { selectedCollection: 'test-collection' }
  })

  const queryInput = screen.getByPlaceholderText('Enter your search query...')
  fireEvent.input(queryInput, { target: { value: 'test query' } })
  fireEvent.keyPress(queryInput, { key: 'Enter' })

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalled()
  })
})

test('displays results with metadata', async () => {
  const mockResponse: QueryResponse = {
    ids: [['id1']],
    distances: [[0.1234]],
    documents: [['test document']],
    metadatas: [[{ source: 'test.pdf', page: 1 }]]
  }

  mockedAxios.post.mockResolvedValueOnce({
    data: {
      success: true,
      data: mockResponse
    }
  })

  render(QueryInterface, {
    props: { selectedCollection: 'test-collection' }
  })

  const queryInput = screen.getByPlaceholderText('Enter your search query...')
  fireEvent.input(queryInput, { target: { value: 'test' } })

  const searchButton = screen.getByText('Search')
  fireEvent.click(searchButton)

  await waitFor(() => {
    expect(screen.getByText('test document')).toBeTruthy()
    expect(screen.getByText(/Distance: 0.1234/)).toBeTruthy()
    expect(screen.getByText(/"source": "test.pdf"/)).toBeTruthy()
  })
})

test('shows no results message when query returns empty', async () => {
  const mockResponse: QueryResponse = {
    ids: [[]],
    distances: [[]],
    documents: [[]],
    metadatas: [[]]
  }

  mockedAxios.post.mockResolvedValueOnce({
    data: {
      success: true,
      data: mockResponse
    }
  })

  render(QueryInterface, {
    props: { selectedCollection: 'test-collection' }
  })

  const queryInput = screen.getByPlaceholderText('Enter your search query...')
  fireEvent.input(queryInput, { target: { value: 'test' } })

  const searchButton = screen.getByText('Search')
  fireEvent.click(searchButton)

  await waitFor(() => {
    expect(screen.getByText('No results found')).toBeTruthy()
  })
})
