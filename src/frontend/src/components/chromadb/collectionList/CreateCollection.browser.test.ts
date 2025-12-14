/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import CreateCollection from './CreateCollection.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import type { ChromaDBCollection } from '@types/chromadb.ts'

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

test('renders create button initially', () => {
  render(CreateCollection)

  const button = screen.getByTitle('Create Collection')
  expect(button).toBeTruthy()
})

test('shows form when button is clicked', async () => {
  render(CreateCollection)

  const button = screen.getByTitle('Create Collection')
  fireEvent.click(button)

  await waitFor(() => {
    expect(screen.getByText('Create New Collection')).toBeTruthy()
  })
})

test('hides form when cancel button is clicked', async () => {
  render(CreateCollection)

  // Open form
  const createButton = screen.getByTitle('Create Collection')
  fireEvent.click(createButton)

  await waitFor(() => {
    expect(screen.getByText('Create New Collection')).toBeTruthy()
  })

  // Close form
  const cancelButton = screen.getByText('Cancel')
  fireEvent.click(cancelButton)

  await waitFor(() => {
    expect(screen.queryByText('Create New Collection')).not.toBeInTheDocument()
  })
})

test('creates collection successfully', async () => {
  const mockCollection: ChromaDBCollection = {
    id: 'new-collection-id',
    name: 'new-collection',
    count: 0
  }

  mockedAxios.post.mockResolvedValueOnce({
    data: {
      success: true,
      data: mockCollection
    }
  })

  render(CreateCollection)

  // Open form
  const createButton = screen.getByTitle('Create Collection')
  fireEvent.click(createButton)

  await waitFor(() => {
    expect(screen.getByText('Create New Collection')).toBeTruthy()
  })

  // Fill form with valid name (no spaces)
  const nameInput = screen.getByPlaceholderText('Enter collection name...')
  fireEvent.input(nameInput, { target: { value: 'new-collection' } })

  // Submit
  const submitButton = screen.getByText('Create Collection')
  fireEvent.click(submitButton)

  await waitFor(
    () => {
      expect(mockedAxios.post).toHaveBeenCalledWith(
        'chromadb/collections',
        expect.objectContaining({
          name: 'new-collection',
          distance_metric: 'cosine'
        })
      )
    },
    { timeout: 2000 }
  )
})

test('shows error when collection creation fails', async () => {
  mockedAxios.post.mockResolvedValueOnce({
    data: {
      success: false,
      error: 'Collection already exists'
    }
  })

  render(CreateCollection)

  // Open form
  const createButton = screen.getByTitle('Create Collection')
  fireEvent.click(createButton)

  await waitFor(() => {
    expect(screen.getByText('Create New Collection')).toBeTruthy()
  })

  // Fill form with valid name (no spaces)
  const nameInput = screen.getByPlaceholderText('Enter collection name...')
  fireEvent.input(nameInput, { target: { value: 'existing-collection' } })

  // Submit
  const submitButton = screen.getByText('Create Collection')
  fireEvent.click(submitButton)

  await waitFor(
    () => {
      expect(screen.getByText(/Collection already exists/)).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('disables submit button when name is empty', async () => {
  render(CreateCollection)

  // Open form
  const createButton = screen.getByTitle('Create Collection')
  fireEvent.click(createButton)

  await waitFor(() => {
    const submitButton = screen.getByText('Create Collection')
    expect(submitButton).toBeDisabled()
  })
})

test('allows adding metadata fields', async () => {
  render(CreateCollection)

  // Open form
  const createButton = screen.getByTitle('Create Collection')
  fireEvent.click(createButton)

  await waitFor(() => {
    expect(screen.getByText('Create New Collection')).toBeTruthy()
  })

  // Add metadata field
  const addFieldButton = screen.getByText('Add Field')
  fireEvent.click(addFieldButton)

  await waitFor(() => {
    expect(screen.getByPlaceholderText('Key')).toBeTruthy()
    expect(screen.getByPlaceholderText('Value')).toBeTruthy()
  })
})

test('allows removing metadata fields', async () => {
  render(CreateCollection)

  // Open form
  const createButton = screen.getByTitle('Create Collection')
  fireEvent.click(createButton)

  await waitFor(() => {
    expect(screen.getByText('Create New Collection')).toBeTruthy()
  })

  // Add metadata field
  const addFieldButton = screen.getByText('Add Field')
  fireEvent.click(addFieldButton)

  await waitFor(() => {
    expect(screen.getByPlaceholderText('Key')).toBeTruthy()
  })

  // Remove metadata field
  const removeButtons = screen.getAllByTitle('Remove field')
  fireEvent.click(removeButtons[0])

  await waitFor(() => {
    expect(screen.queryByPlaceholderText('Key')).not.toBeInTheDocument()
  })
})

test('allows changing distance metric', async () => {
  render(CreateCollection)

  // Open form
  const createButton = screen.getByTitle('Create Collection')
  fireEvent.click(createButton)

  await waitFor(() => {
    expect(screen.getByText('Create New Collection')).toBeTruthy()
  })

  const select = screen.getByLabelText(/Distance Metric/) as HTMLSelectElement
  fireEvent.change(select, { target: { value: 'l2' } })

  expect(select.value).toBe('l2')
})
