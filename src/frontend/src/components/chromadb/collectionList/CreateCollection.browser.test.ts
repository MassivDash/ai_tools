/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import CreateCollection from './CreateCollection.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import { collections, selectedCollection } from '@stores/chromadb.ts'
import { get } from 'svelte/store'
import type { ChromaDBCollection } from '@types/chromadb.ts'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    post: vi.fn(),
    // CreateCollection.svelte's loadModels() calls .get() on open; default
    // to an empty model list so opening the form doesn't throw/log noise.
    get: vi.fn().mockResolvedValue({ data: { models: [] } })
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  post: ReturnType<typeof vi.fn>
  get: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(console, 'error').mockImplementation(() => {})
  collections.set([])
  selectedCollection.set(null)
})

afterEach(() => {
  vi.restoreAllMocks()
})

const openForm = async () => {
  await fireEvent.click(screen.getByTitle('Create Collection'))
  await waitFor(() => {
    expect(screen.getByText('Create New Collection')).toBeTruthy()
  })
}

const nameField = () =>
  screen.getByPlaceholderText('Enter collection name...') as HTMLInputElement
const submit = () => screen.getByText('Create Collection')
const errorText = () => document.querySelector('.error-message')?.textContent

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

test('lists the available embedding models and preselects the first', async () => {
  mockedAxios.get.mockResolvedValueOnce({
    data: { models: [{ name: 'nomic-embed-text' }, { name: 'all-minilm' }] }
  })

  render(CreateCollection)
  await openForm()

  const select = await waitFor(() => {
    const el = screen.getByLabelText(/Embedding Model/) as HTMLSelectElement
    expect(el.options).toHaveLength(2)
    return el
  })
  expect(mockedAxios.get).toHaveBeenCalledWith('chromadb/models')
  expect(select.value).toBe('nomic-embed-text')
  expect(Array.from(select.options).map((o) => o.textContent?.trim())).toEqual([
    'nomic-embed-text',
    'all-minilm'
  ])
})

test('shows the no-models option when ollama returns nothing', async () => {
  render(CreateCollection)
  await openForm()

  await waitFor(() => {
    expect(screen.getByText('No models found')).toBeTruthy()
  })
})

test('only loads the model list once across form toggles', async () => {
  mockedAxios.get.mockResolvedValueOnce({
    data: { models: [{ name: 'nomic-embed-text' }] }
  })

  render(CreateCollection)
  await openForm()

  await waitFor(() => {
    expect(mockedAxios.get).toHaveBeenCalledTimes(1)
  })

  await fireEvent.click(screen.getByText('Cancel'))
  await waitFor(() => {
    expect(screen.queryByText('Create New Collection')).not.toBeInTheDocument()
  })
  await openForm()

  // models are cached, so no second request
  expect(mockedAxios.get).toHaveBeenCalledTimes(1)
  const select = screen.getByLabelText(/Embedding Model/) as HTMLSelectElement
  expect(select.value).toBe('nomic-embed-text')
})

test('logs and keeps the form usable when the model list fails to load', async () => {
  mockedAxios.get.mockRejectedValueOnce(new Error('ollama down'))

  render(CreateCollection)
  await openForm()

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to load models:',
      expect.any(Error)
    )
  })
  expect(screen.getByText('No models found')).toBeTruthy()
  expect(nameField()).not.toBeDisabled()
})

test('sends metadata and the selected embedding model, then resets the form', async () => {
  mockedAxios.get.mockResolvedValueOnce({
    data: { models: [{ name: 'nomic-embed-text' }, { name: 'all-minilm' }] }
  })
  const created: ChromaDBCollection = {
    id: 'created-id',
    name: 'docs-2026',
    count: 0
  }
  mockedAxios.post.mockResolvedValueOnce({
    data: { success: true, data: created }
  })

  render(CreateCollection)
  await openForm()

  await waitFor(() => {
    expect(
      (screen.getByLabelText(/Embedding Model/) as HTMLSelectElement).options
    ).toHaveLength(2)
  })

  await fireEvent.change(screen.getByLabelText(/Embedding Model/), {
    target: { value: 'all-minilm' }
  })
  await fireEvent.change(screen.getByLabelText(/Distance Metric/), {
    target: { value: 'ip' }
  })
  await fireEvent.input(nameField(), { target: { value: 'docs-2026' } })

  await fireEvent.click(screen.getByText('Add Field'))
  await waitFor(() => {
    expect(screen.getByPlaceholderText('Key')).toBeTruthy()
  })
  await fireEvent.input(screen.getByPlaceholderText('Key'), {
    target: { value: 'owner' }
  })
  await fireEvent.input(screen.getByPlaceholderText('Value'), {
    target: { value: 'research' }
  })

  await fireEvent.click(submit())

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('chromadb/collections', {
      name: 'docs-2026',
      metadata: { owner: 'research' },
      distance_metric: 'ip',
      embedding_model: 'all-minilm'
    })
  })

  // store side effects
  await waitFor(() => {
    expect(get(collections)).toEqual([created])
  })
  expect(get(selectedCollection)).toEqual(created)

  // the form closes and is reset back to the first model / cosine
  await waitFor(() => {
    expect(screen.queryByText('Create New Collection')).not.toBeInTheDocument()
  })
  await openForm()
  expect(nameField().value).toBe('')
  expect(
    (screen.getByLabelText(/Embedding Model/) as HTMLSelectElement).value
  ).toBe('nomic-embed-text')
  expect(
    (screen.getByLabelText(/Distance Metric/) as HTMLSelectElement).value
  ).toBe('cosine')
  expect(
    screen.getByText('No metadata fields. Click "Add Field" to add some.')
  ).toBeTruthy()
})

test('renames a metadata key in place and keeps its value', async () => {
  mockedAxios.post.mockResolvedValueOnce({
    data: { success: true, data: { id: 'x', name: 'renamed', count: 0 } }
  })

  render(CreateCollection)
  await openForm()

  await fireEvent.click(screen.getByText('Add Field'))
  await waitFor(() => {
    expect(screen.getByPlaceholderText('Key')).toBeTruthy()
  })
  expect((screen.getByPlaceholderText('Key') as HTMLInputElement).value).toBe(
    'key_1'
  )

  await fireEvent.input(screen.getByPlaceholderText('Value'), {
    target: { value: 'confidential' }
  })
  await fireEvent.input(screen.getByPlaceholderText('Key'), {
    target: { value: 'classification' }
  })

  await waitFor(() => {
    expect((screen.getByPlaceholderText('Key') as HTMLInputElement).value).toBe(
      'classification'
    )
  })
  expect((screen.getByPlaceholderText('Value') as HTMLInputElement).value).toBe(
    'confidential'
  )
  // still a single field: the old key was removed, not duplicated
  expect(document.querySelectorAll('.metadata-field')).toHaveLength(1)

  await fireEvent.input(nameField(), { target: { value: 'renamed' } })
  await fireEvent.click(submit())

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'chromadb/collections',
      expect.objectContaining({
        metadata: { classification: 'confidential' }
      })
    )
  })
})

test('ignores a metadata key edit that does not change the key', async () => {
  render(CreateCollection)
  await openForm()

  await fireEvent.click(screen.getByText('Add Field'))
  await waitFor(() => {
    expect(screen.getByPlaceholderText('Key')).toBeTruthy()
  })

  await fireEvent.input(screen.getByPlaceholderText('Key'), {
    target: { value: 'key_1' }
  })

  expect(document.querySelectorAll('.metadata-field')).toHaveLength(1)
  expect((screen.getByPlaceholderText('Key') as HTMLInputElement).value).toBe(
    'key_1'
  )
})

test('numbers additional metadata fields sequentially', async () => {
  render(CreateCollection)
  await openForm()

  await fireEvent.click(screen.getByText('Add Field'))
  await waitFor(() => {
    expect(screen.getAllByPlaceholderText('Key')).toHaveLength(1)
  })
  await fireEvent.click(screen.getByText('Add Field'))

  await waitFor(() => {
    expect(screen.getAllByPlaceholderText('Key')).toHaveLength(2)
  })
  expect(
    screen
      .getAllByPlaceholderText('Key')
      .map((el) => (el as HTMLInputElement).value)
  ).toEqual(['key_1', 'key_2'])
})

test('rejects an invalid collection name without calling the API', async () => {
  render(CreateCollection)
  await openForm()

  await fireEvent.input(nameField(), { target: { value: 'bad name!' } })
  await fireEvent.click(submit())

  await waitFor(() => {
    expect(errorText()).toBe(
      'Collection name can only contain alphanumeric characters, underscores, and hyphens'
    )
  })
  expect(mockedAxios.post).not.toHaveBeenCalled()
  // form stays open and the button is usable again
  expect(submit()).not.toBeDisabled()
})

test('rejects a name longer than 100 characters without calling the API', async () => {
  render(CreateCollection)
  await openForm()

  await fireEvent.input(nameField(), { target: { value: 'a'.repeat(101) } })
  await fireEvent.click(submit())

  await waitFor(() => {
    expect(errorText()).toBe('Collection name is too long (max 100 characters)')
  })
  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('falls back to a generic message when the API reports failure with no error', async () => {
  mockedAxios.post.mockResolvedValueOnce({ data: { success: false } })

  render(CreateCollection)
  await openForm()

  await fireEvent.input(nameField(), { target: { value: 'my-collection' } })
  await fireEvent.click(submit())

  await waitFor(() => {
    expect(errorText()).toBe('Failed to create collection')
  })
  expect(get(collections)).toEqual([])
})

test('shows the backend error payload when the request throws', async () => {
  mockedAxios.post.mockRejectedValueOnce({
    response: { data: { error: 'chroma is not running' } }
  })

  render(CreateCollection)
  await openForm()

  await fireEvent.input(nameField(), { target: { value: 'my-collection' } })
  await fireEvent.click(submit())

  await waitFor(() => {
    expect(errorText()).toBe('chroma is not running')
  })
  expect(console.error).toHaveBeenCalledWith(
    'Error creating collection:',
    expect.anything()
  )
})

test('falls back to the thrown message, then a generic one, on request failure', async () => {
  mockedAxios.post.mockRejectedValueOnce(new Error('timeout of 5000ms'))

  const { unmount } = render(CreateCollection)
  await openForm()
  await fireEvent.input(nameField(), { target: { value: 'my-collection' } })
  await fireEvent.click(submit())

  await waitFor(() => {
    expect(errorText()).toBe('timeout of 5000ms')
  })

  unmount()
  mockedAxios.post.mockRejectedValueOnce({})
  render(CreateCollection)
  await openForm()
  await fireEvent.input(nameField(), { target: { value: 'my-collection' } })
  await fireEvent.click(submit())

  await waitFor(() => {
    expect(errorText()).toBe('Failed to create collection')
  })
})

test('shows a creating label while the request is in flight', async () => {
  let release: (_value: unknown) => void = () => {}
  mockedAxios.post.mockReturnValueOnce(
    new Promise((resolve) => {
      release = resolve
    })
  )

  render(CreateCollection)
  await openForm()

  await fireEvent.input(nameField(), { target: { value: 'my-collection' } })
  await fireEvent.click(submit())

  await waitFor(() => {
    expect(screen.getByText('Creating...')).toBeDisabled()
  })
  expect(nameField()).toBeDisabled()
  expect(screen.getByText('Cancel')).toBeDisabled()

  release({ data: { success: true, data: { id: '1', name: 'my-collection' } } })

  await waitFor(() => {
    expect(screen.queryByText('Creating...')).not.toBeInTheDocument()
  })
})
