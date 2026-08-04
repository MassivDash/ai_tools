/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import ChromaDBConfig from './ChromaDBConfig.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import type { ModelNote } from '@types'

vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    get: vi.fn(),
    post: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
}

type ModelInfo = { name: string; size?: string; modified?: string }

const defaultConfig = {
  embedding_model: 'nomic-embed-text',
  query_model: 'nomic-embed-text',
  chunk_size: 384,
  chunk_overlap: 50
}

/**
 * Wires up the three GET endpoints ChromaDBConfig hits when it opens.
 * Each entry may be a value (resolved) or a thenable (to reject / hang).
 */
const stubGets = (
  overrides: {
    config?: unknown
    models?: unknown
    notes?: unknown
  } = {}
) => {
  const resolveOr = (value: unknown, fallback: unknown) => {
    if (value === undefined) return Promise.resolve({ data: fallback })
    if (value instanceof Promise) return value
    return Promise.resolve({ data: value })
  }

  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'chromadb/config') {
      return resolveOr(overrides.config, defaultConfig)
    }
    if (url === 'chromadb/models') {
      return resolveOr(overrides.models, { models: [] })
    }
    if (url === 'model-notes') {
      return resolveOr(overrides.notes, { notes: [] })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })
}

const openProps = (extra: Record<string, unknown> = {}) => ({
  isOpen: true,
  onClose: vi.fn(),
  onSave: vi.fn(),
  ...extra
})

const modelInput = () =>
  screen.getByLabelText('Embedding Model') as HTMLInputElement
const chunkSizeInput = () =>
  screen.getByLabelText('Chunk Size (Tokens)') as HTMLInputElement
const overlapInput = () =>
  screen.getByLabelText('Overlap (Tokens)') as HTMLInputElement
const saveButton = () => screen.getByRole('button', { name: 'Save' })

const note = (
  partial: Partial<ModelNote> & { model_name: string }
): ModelNote =>
  ({
    platform: 'ollama',
    is_favorite: false,
    is_default: false,
    tags: [],
    ...partial
  }) as ModelNote

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(console, 'error').mockImplementation(() => {})
  stubGets()
})

afterEach(() => {
  vi.restoreAllMocks()
})

test('does not hit the backend and stays hidden while closed', async () => {
  const { container } = render(ChromaDBConfig, {
    props: { isOpen: false, onClose: vi.fn(), onSave: vi.fn() }
  })

  expect(container.querySelector('.config-panel')).not.toHaveClass('visible')
  await Promise.resolve()
  expect(mockedAxios.get).not.toHaveBeenCalled()
})

test('loads the saved config into the form when opened', async () => {
  stubGets({
    config: {
      embedding_model: 'mxbai-embed-large',
      query_model: 'mxbai-embed-large',
      chunk_size: 512,
      chunk_overlap: 64
    }
  })

  const { container } = render(ChromaDBConfig, { props: openProps() })

  expect(container.querySelector('.config-panel')).toHaveClass('visible')

  await waitFor(() => {
    expect(modelInput().value).toBe('mxbai-embed-large')
  })
  expect(chunkSizeInput().value).toBe('512')
  expect(overlapInput().value).toBe('64')
  expect(mockedAxios.get).toHaveBeenCalledWith('chromadb/config')
  expect(mockedAxios.get).toHaveBeenCalledWith('chromadb/models')
  expect(mockedAxios.get).toHaveBeenCalledWith('model-notes')
})

test('falls back to default chunking when the backend returns zeroes', async () => {
  stubGets({
    config: {
      embedding_model: 'custom-embedder',
      query_model: 'custom-embedder',
      chunk_size: 0,
      chunk_overlap: 0
    }
  })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(modelInput().value).toBe('custom-embedder')
  })
  expect(chunkSizeInput().value).toBe('384')
  expect(overlapInput().value).toBe('50')
})

test('keeps defaults and logs when the config request fails', async () => {
  stubGets({ config: Promise.reject(new Error('config down')) })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to load config:',
      expect.any(Error)
    )
  })
  expect(modelInput().value).toBe('nomic-embed-text')
  expect(chunkSizeInput().value).toBe('384')
  // A failed config load must not surface as a user-visible error banner
  expect(document.querySelector('.error')).toBeNull()
})

test('renders the model list with size/modified subtext', async () => {
  const models: ModelInfo[] = [
    { name: 'nomic-embed-text', size: '274 MB', modified: '2 days ago' },
    { name: 'sized-only', size: '1.2 GB' },
    { name: 'bare-model' }
  ]
  stubGets({ models: { models } })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(screen.getByText('nomic-embed-text')).toBeTruthy()
  })
  expect(screen.getByText('274 MB • 2 days ago')).toBeTruthy()
  expect(screen.getByText('1.2 GB')).toBeTruthy()
  // bare-model has neither size nor modified -> empty subtext
  const bareItem = screen
    .getByText('bare-model')
    .closest('button') as HTMLButtonElement
  expect(bareItem.querySelector('.item-subtext')?.textContent).toBe('')
})

test('still renders a model that has no name', async () => {
  stubGets({ models: { models: [{ name: 'nomic-embed-text' }, { name: '' }] } })

  const { container } = render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(container.querySelectorAll('.list-item')).toHaveLength(2)
  })
})

test('shows the loading placeholder while the model list is in flight', async () => {
  stubGets({ models: new Promise(() => {}) })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(screen.getByText('Loading models...')).toBeTruthy()
  })
  expect(document.querySelector('.searchable-list')).toBeNull()
})

test('shows the empty state when Ollama has no models', async () => {
  stubGets({ models: { models: [] } })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(screen.getByText('No Ollama models found')).toBeTruthy()
  })
  expect(
    screen.getByText("Run 'ollama pull <model>' to download models")
  ).toBeTruthy()
})

test('shows the backend error payload when loading models fails', async () => {
  stubGets({
    models: Promise.reject({ response: { data: { error: 'ollama offline' } } })
  })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(screen.getByText('ollama offline')).toBeTruthy()
  })
  expect(document.querySelector('.error')?.textContent).toBe('ollama offline')
})

test('falls back to the thrown error message when loading models fails', async () => {
  stubGets({ models: Promise.reject(new Error('network unreachable')) })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toBe(
      'network unreachable'
    )
  })
})

test('falls back to a generic message when the model error is empty', async () => {
  stubGets({ models: Promise.reject({}) })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toBe(
      'Failed to load models'
    )
  })
})

test('decorates ollama models with favourite, tags and truncated notes', async () => {
  const longNote = 'z'.repeat(150)
  stubGets({
    models: {
      models: [
        { name: 'nomic-embed-text' },
        { name: 'all-minilm' },
        { name: 'unannotated-model' }
      ]
    },
    notes: {
      notes: [
        note({
          model_name: 'nomic-embed-text',
          is_favorite: true,
          tags: ['fast', 'default'],
          notes: longNote
        }),
        note({
          model_name: 'all-minilm',
          is_favorite: false,
          tags: undefined as unknown as string[],
          notes: 'short note'
        }),
        // non-ollama notes must be ignored entirely
        note({
          platform: 'llamacpp',
          model_name: 'unannotated-model',
          is_favorite: true,
          tags: ['ignored-tag'],
          notes: 'ignored note'
        }),
        // ollama note without a model name cannot be keyed and is dropped
        note({ model_name: '', is_favorite: true, tags: ['nameless'] })
      ]
    }
  })

  const { container } = render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(screen.getByText('fast')).toBeTruthy()
  })
  expect(screen.getByText('default')).toBeTruthy()
  expect(screen.queryByText('ignored-tag')).not.toBeInTheDocument()
  expect(screen.queryByText('nameless')).not.toBeInTheDocument()
  expect(screen.queryByText('ignored note')).not.toBeInTheDocument()

  // > 100 chars gets truncated with an ellipsis, shorter notes are shown as-is
  expect(screen.getByText(`${'z'.repeat(100)}...`)).toBeTruthy()
  expect(screen.getByText('short note')).toBeTruthy()

  // only the favourited ollama model gets a star
  expect(container.querySelectorAll('.favorite-icon')).toHaveLength(1)
  const favouriteItem = screen
    .getByText('nomic-embed-text')
    .closest('button') as HTMLButtonElement
  expect(favouriteItem.querySelector('.favorite-icon')).not.toBeNull()

  // a model with no matching note renders neither tags nor notes
  const plainItem = screen
    .getByText('unannotated-model')
    .closest('button') as HTMLButtonElement
  expect(plainItem.querySelector('.item-tags')).toBeNull()
  expect(plainItem.querySelector('.item-notes')).toBeNull()
})

test('ignores a failing model-notes request', async () => {
  stubGets({
    models: { models: [{ name: 'nomic-embed-text' }] },
    notes: Promise.reject(new Error('notes table missing'))
  })

  const { container } = render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to load model notes:',
      expect.any(Error)
    )
  })
  expect(screen.getByText('nomic-embed-text')).toBeTruthy()
  expect(container.querySelector('.favorite-icon')).toBeNull()
  expect(document.querySelector('.error')).toBeNull()
})

test('marks the model matching the loaded config as selected', async () => {
  stubGets({
    models: { models: [{ name: 'nomic-embed-text' }, { name: 'all-minilm' }] }
  })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(screen.getByText('nomic-embed-text').closest('button')).toHaveClass(
      'selected'
    )
  })
  expect(screen.getByText('all-minilm').closest('button')).not.toHaveClass(
    'selected'
  )
})

test('marks nothing as selected when the configured model is not installed', async () => {
  stubGets({
    config: { ...defaultConfig, embedding_model: 'not-installed' },
    models: { models: [{ name: 'nomic-embed-text' }, { name: 'all-minilm' }] }
  })

  const { container } = render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(modelInput().value).toBe('not-installed')
  })
  expect(container.querySelector('.list-item.selected')).toBeNull()
})

test('applies the recommended chunk settings for the picked model', async () => {
  stubGets({
    config: { ...defaultConfig, chunk_size: 1024, chunk_overlap: 128 },
    models: {
      models: [
        { name: 'all-minilm' },
        { name: 'some-other-embedder' },
        { name: 'mxbai-embed-large' }
      ]
    }
  })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(screen.getByText('all-minilm')).toBeTruthy()
  })
  expect(chunkSizeInput().value).toBe('1024')

  // minilm -> smaller context
  await fireEvent.click(screen.getByText('all-minilm').closest('button')!)
  await waitFor(() => {
    expect(modelInput().value).toBe('all-minilm')
  })
  expect(chunkSizeInput().value).toBe('256')
  expect(overlapInput().value).toBe('30')

  // unknown model -> default safe values
  await fireEvent.click(
    screen.getByText('some-other-embedder').closest('button')!
  )
  await waitFor(() => {
    expect(modelInput().value).toBe('some-other-embedder')
  })
  expect(chunkSizeInput().value).toBe('384')
  expect(overlapInput().value).toBe('50')

  // mxbai -> 512-context safe values
  await fireEvent.input(chunkSizeInput(), { target: { value: '999' } })
  await fireEvent.click(
    screen.getByText('mxbai-embed-large').closest('button')!
  )
  await waitFor(() => {
    expect(modelInput().value).toBe('mxbai-embed-large')
  })
  expect(chunkSizeInput().value).toBe('384')
  expect(overlapInput().value).toBe('50')
})

test('filters the model list through the search box', async () => {
  stubGets({
    models: { models: [{ name: 'nomic-embed-text' }, { name: 'all-minilm' }] }
  })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(screen.getByText('all-minilm')).toBeTruthy()
  })

  await fireEvent.input(screen.getByPlaceholderText('Search models...'), {
    target: { value: 'minilm' }
  })

  await waitFor(() => {
    expect(screen.queryByText('nomic-embed-text')).not.toBeInTheDocument()
  })
  expect(screen.getByText('all-minilm')).toBeTruthy()
})

test('saves the edited config, reloads it and closes the panel', async () => {
  const props = openProps()
  mockedAxios.post.mockResolvedValue({
    data: { success: true, message: 'Configuration updated' }
  })

  render(ChromaDBConfig, { props })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.input(modelInput(), {
    target: { value: 'mxbai-embed-large' }
  })
  await fireEvent.input(chunkSizeInput(), { target: { value: '256' } })
  await fireEvent.input(overlapInput(), { target: { value: '32' } })

  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('chromadb/config', {
      embedding_model: 'mxbai-embed-large',
      chunk_size: 256,
      chunk_overlap: 32
    })
  })
  // query_model is deliberately omitted from the payload
  expect(mockedAxios.post.mock.calls[0][1]).not.toHaveProperty('query_model')

  await waitFor(() => {
    expect(props.onSave).toHaveBeenCalledTimes(1)
  })
  expect(props.onClose).toHaveBeenCalledTimes(1)
  // config is re-fetched after a successful save (once on open + once after)
  expect(
    mockedAxios.get.mock.calls.filter((c) => c[0] === 'chromadb/config')
  ).toHaveLength(2)
  expect(document.querySelector('.error')).toBeNull()
})

test('trims the embedding model before sending it', async () => {
  mockedAxios.post.mockResolvedValue({
    data: { success: true, message: 'ok' }
  })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.input(modelInput(), { target: { value: '  spaced-model  ' } })
  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'chromadb/config',
      expect.objectContaining({ embedding_model: 'spaced-model' })
    )
  })
})

test('shows a validation error and skips the request for a zero chunk size', async () => {
  const props = openProps()
  render(ChromaDBConfig, { props })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.input(chunkSizeInput(), { target: { value: '0' } })
  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toMatch(/Too small/)
  })
  expect(mockedAxios.post).not.toHaveBeenCalled()
  expect(props.onSave).not.toHaveBeenCalled()
  expect(props.onClose).not.toHaveBeenCalled()
  // the button is re-enabled so the user can fix the value
  expect(saveButton()).not.toBeDisabled()
})

test('shows a validation error and skips the request for a negative overlap', async () => {
  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.input(overlapInput(), { target: { value: '-5' } })
  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toMatch(/Too small/)
  })
  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('shows a validation error and skips the request for a fractional chunk size', async () => {
  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.input(chunkSizeInput(), { target: { value: '12.5' } })
  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toMatch(
      /expected int/
    )
  })
  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('surfaces the API message when the save is refused', async () => {
  const props = openProps()
  mockedAxios.post.mockResolvedValue({
    data: { success: false, message: 'embedding model not pulled' }
  })

  render(ChromaDBConfig, { props })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toBe(
      'embedding model not pulled'
    )
  })
  expect(props.onSave).not.toHaveBeenCalled()
  expect(props.onClose).not.toHaveBeenCalled()
})

test('surfaces the backend error payload when the save request throws', async () => {
  mockedAxios.post.mockRejectedValue({
    response: { data: { error: 'chroma unreachable' } }
  })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toBe(
      'chroma unreachable'
    )
  })
})

test('falls back to the thrown message, then a generic message, on save failure', async () => {
  mockedAxios.post.mockRejectedValueOnce(new Error('socket hang up'))

  const { unmount } = render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })
  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toBe('socket hang up')
  })

  unmount()
  mockedAxios.post.mockRejectedValueOnce({})
  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })
  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toBe(
      'Failed to save config'
    )
  })
})

test('disables the save button and shows progress while saving', async () => {
  let releasePost: (_value: unknown) => void = () => {}
  mockedAxios.post.mockReturnValue(
    new Promise((resolve) => {
      releasePost = resolve
    })
  )

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.click(saveButton())

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Saving...' })).toBeDisabled()
  })

  releasePost({ data: { success: true, message: 'ok' } })

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Save' })).toBeTruthy()
  })
})

test('clears a previous error when a later save succeeds', async () => {
  mockedAxios.post.mockRejectedValueOnce(new Error('transient failure'))
  mockedAxios.post.mockResolvedValueOnce({
    data: { success: true, message: 'ok' }
  })
  const props = openProps()

  render(ChromaDBConfig, { props })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.click(saveButton())
  await waitFor(() => {
    expect(document.querySelector('.error')?.textContent).toBe(
      'transient failure'
    )
  })

  await fireEvent.click(saveButton())
  await waitFor(() => {
    expect(props.onSave).toHaveBeenCalledTimes(1)
  })
  expect(document.querySelector('.error')).toBeNull()
})

test('disables the save button while the embedding model is blank', async () => {
  stubGets({ config: { ...defaultConfig, embedding_model: '   ' } })

  render(ChromaDBConfig, { props: openProps() })

  await waitFor(() => {
    expect(saveButton()).toBeDisabled()
  })
  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('cancel and close both call onClose without saving', async () => {
  const props = openProps()
  render(ChromaDBConfig, { props })

  await waitFor(() => {
    expect(modelInput().value).toBe('nomic-embed-text')
  })

  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
  expect(props.onClose).toHaveBeenCalledTimes(1)

  await fireEvent.click(screen.getByTitle('Close'))
  expect(props.onClose).toHaveBeenCalledTimes(2)
  expect(mockedAxios.post).not.toHaveBeenCalled()
})
