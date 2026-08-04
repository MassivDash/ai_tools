/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import LlamaConfig from './LlamaConfig.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import type { Component } from 'svelte'

// Mock axiosBackendInstance
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

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

type GetResponses = Record<string, unknown>

class Rejection {
  reason: unknown
  constructor(reason: unknown) {
    this.reason = reason
  }
}

const rejectWith = (reason: unknown) => new Rejection(reason)

// Answers every endpoint the panel calls on open; pass overrides per test.
// Values are resolved as `{ data: value }` unless wrapped in `rejectWith`.
const mockGets = (overrides: GetResponses = {}) => {
  const responses: GetResponses = {
    'llama-server/config': { hf_model: '', ctx_size: 0 },
    'llama-server/models': { local_models: [] },
    'agent/config': { debug_logging: false },
    'model-notes': { notes: [] },
    ...overrides
  }

  mockedAxios.get.mockImplementation((url: string) => {
    if (!(url in responses)) {
      return Promise.reject(new Error(`Unexpected URL: ${url}`))
    }
    const value = responses[url]
    if (value instanceof Rejection) {
      return Promise.reject(value.reason)
    }
    return Promise.resolve({ data: value })
  })
}

const openPanel = async (props: Record<string, unknown> = {}) => {
  const rendered = render(LlamaConfig as Component, {
    props: { isOpen: true, onClose: vi.fn(), ...props }
  })
  await waitFor(() => {
    expect(screen.getByText('Server Configuration')).toBeTruthy()
  })
  return rendered
}

const openAdvancedOptions = async () => {
  fireEvent.click(screen.getByText('Advanced Options'))
  await waitFor(() => {
    expect(screen.getByLabelText('Model Path')).toBeTruthy()
  })
}

const hfModelInput = () =>
  screen.getByPlaceholderText(
    /e.g., unsloth\/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL/
  ) as HTMLInputElement

test('renders config panel when open', () => {
  const onClose = vi.fn()
  const onSave = vi.fn()

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose,
      onSave
    }
  })

  expect(screen.getByText('Server Configuration')).toBeTruthy()
})

test('does not render when closed', () => {
  const onClose = vi.fn()
  const onSave = vi.fn()

  const { container } = render(LlamaConfig as Component, {
    props: {
      isOpen: false,
      onClose,
      onSave
    }
  })

  // Component uses class-based visibility, so it's still in DOM but not visible
  const configPanel = container.querySelector('.config-panel')
  expect(configPanel).toBeTruthy()
  expect(configPanel).not.toHaveClass('visible')
})

test('loads config and models on open', async () => {
  const mockConfig = {
    hf_model: 'test-model',
    ctx_size: 2048,
    threads: 4,
    threads_batch: 2,
    predict: 100,
    batch_size: 512,
    ubatch_size: 256,
    flash_attn: true,
    mlock: false,
    no_mmap: false,
    gpu_layers: 10,
    model: '/path/to/model.gguf'
  }

  const mockModels = {
    local_models: [
      {
        name: 'Model 1',
        path: '/path/to/model1.gguf',
        size: 1024000,
        hf_format: 'model1'
      },
      {
        name: 'Model 2',
        path: '/path/to/model2.gguf',
        size: 2048000
      }
    ]
  }

  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({ data: mockConfig })
    }
    if (url === 'llama-server/models') {
      return Promise.resolve({ data: mockModels })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  const onClose = vi.fn()
  const onSave = vi.fn()

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose,
      onSave
    }
  })

  await waitFor(
    () => {
      expect(mockedAxios.get).toHaveBeenCalledWith('llama-server/config')
      expect(mockedAxios.get).toHaveBeenCalledWith('llama-server/models')
    },
    { timeout: 2000 }
  )

  await waitFor(
    () => {
      const input = screen.getByPlaceholderText(
        /e.g., unsloth\/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL/
      ) as HTMLInputElement
      expect(input.value).toBe('test-model')
    },
    { timeout: 2000 }
  )
})

test('displays local models in searchable list', async () => {
  const mockModels = {
    local_models: [
      {
        name: 'Model 1',
        path: '/path/to/model1.gguf',
        size: 1024000,
        hf_format: 'model1'
      }
    ]
  }

  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({
        data: { hf_model: '', ctx_size: 0 }
      })
    }
    if (url === 'llama-server/models') {
      return Promise.resolve({ data: mockModels })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn()
    }
  })

  await waitFor(
    () => {
      expect(screen.getByText('Model 1')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('shows loading state when loading models', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({
        data: { hf_model: '', ctx_size: 0 }
      })
    }
    if (url === 'llama-server/models') {
      return new Promise(() => {}) // Never resolves
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn()
    }
  })

  await waitFor(
    () => {
      expect(screen.getByText('Loading models...')).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('shows empty state when no models found', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({
        data: { hf_model: '', ctx_size: 0 }
      })
    }
    if (url === 'llama-server/models') {
      return Promise.resolve({ data: { local_models: [] } })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn()
    }
  })

  await waitFor(
    () => {
      expect(
        screen.getByText(/No GGUF models found in ~\/\.cache\/llama\.cpp\//)
      ).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('allows selecting model from list', async () => {
  const mockModels = {
    local_models: [
      {
        name: 'Model 1',
        path: '/path/to/model1.gguf',
        hf_format: 'model1'
      }
    ]
  }

  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({
        data: { hf_model: '', ctx_size: 0 }
      })
    }
    if (url === 'llama-server/models') {
      return Promise.resolve({ data: mockModels })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn()
    }
  })

  await waitFor(
    () => {
      expect(screen.getByText('Model 1')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // Find the model item and click it
  // The SearchableList component dispatches a select event
  const modelItem = screen.getByText('Model 1')
  const listItem =
    modelItem.closest('div[role="button"]') ||
    modelItem.closest('.searchable-list-item')

  if (listItem) {
    fireEvent.click(listItem)

    await waitFor(
      () => {
        const input = screen.getByPlaceholderText(
          /e.g., unsloth\/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL/
        ) as HTMLInputElement
        // The value might be model1 or the path, depending on which is used
        expect(input.value.length).toBeGreaterThan(0)
      },
      { timeout: 2000 }
    )
  }
})

test('saves config successfully', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({
        data: { hf_model: '', ctx_size: 0 }
      })
    }
    if (url === 'llama-server/models') {
      return Promise.resolve({ data: { local_models: [] } })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  mockedAxios.post.mockResolvedValueOnce({
    data: { success: true, message: 'Config saved' }
  })

  // Mock the second get call after save
  mockedAxios.get.mockResolvedValueOnce({
    data: { hf_model: 'test-model', ctx_size: 0 }
  })

  const onClose = vi.fn()
  const onSave = vi.fn()

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose,
      onSave
    }
  })

  await waitFor(
    () => {
      expect(screen.getByText('Server Configuration')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // Fill in model name
  const modelInput = screen.getByPlaceholderText(
    /e.g., unsloth\/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL/
  ) as HTMLInputElement
  fireEvent.input(modelInput, { target: { value: 'test-model' } })

  // Wait for input to update
  await waitFor(
    () => {
      expect(modelInput.value).toBe('test-model')
    },
    { timeout: 2000 }
  )

  // Click save
  const saveButton = screen.getByText('Save')
  expect(saveButton).not.toBeDisabled()
  fireEvent.click(saveButton)

  await waitFor(
    () => {
      expect(mockedAxios.post).toHaveBeenCalledWith(
        'llama-server/config',
        expect.objectContaining({
          hf_model: 'test-model'
        })
      )
    },
    { timeout: 2000 }
  )

  await waitFor(
    () => {
      expect(onSave).toHaveBeenCalled()
      expect(onClose).toHaveBeenCalled()
    },
    { timeout: 2000 }
  )
})

test('shows error when save fails', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({
        data: { hf_model: '', ctx_size: 0 }
      })
    }
    if (url === 'llama-server/models') {
      return Promise.resolve({ data: { local_models: [] } })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  mockedAxios.post.mockResolvedValueOnce({
    data: { success: false, message: 'Failed to save config' }
  })

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn()
    }
  })

  await waitFor(
    () => {
      expect(screen.getByText('Server Configuration')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // Fill in model name
  const modelInput = screen.getByPlaceholderText(
    /e.g., unsloth\/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL/
  ) as HTMLInputElement
  fireEvent.input(modelInput, { target: { value: 'test-model' } })

  await waitFor(
    () => {
      expect(modelInput.value).toBe('test-model')
    },
    { timeout: 2000 }
  )

  // Click save
  const saveButton = screen.getByText('Save')
  fireEvent.click(saveButton)

  await waitFor(
    () => {
      expect(screen.getByText(/Failed to save config/)).toBeTruthy()
    },
    { timeout: 2000 }
  )
})

test('disables save button when model name is empty', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({
        data: { hf_model: '', ctx_size: 0 }
      })
    }
    if (url === 'llama-server/models') {
      return Promise.resolve({ data: { local_models: [] } })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn()
    }
  })

  await waitFor(
    () => {
      const saveButton = screen.getByText('Save')
      expect(saveButton).toBeDisabled()
    },
    { timeout: 2000 }
  )
})

test('allows changing context size', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({
        data: { hf_model: 'test-model', ctx_size: 0 }
      })
    }
    if (url === 'llama-server/models') {
      return Promise.resolve({ data: { local_models: [] } })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn()
    }
  })

  await waitFor(
    () => {
      const ctxInput = screen.getByLabelText(/Context Size/) as HTMLInputElement
      expect(ctxInput.value).toBe('0')
    },
    { timeout: 2000 }
  )

  const ctxInput = screen.getByLabelText(/Context Size/) as HTMLInputElement
  fireEvent.input(ctxInput, { target: { value: '2048' } })

  expect(ctxInput.value).toBe('2048')
})

test('allows toggling advanced options accordion', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'llama-server/config') {
      return Promise.resolve({
        data: { hf_model: 'test-model', ctx_size: 0 }
      })
    }
    if (url === 'llama-server/models') {
      return Promise.resolve({ data: { local_models: [] } })
    }
    return Promise.reject(new Error(`Unexpected URL: ${url}`))
  })

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn()
    }
  })

  await waitFor(
    () => {
      expect(screen.getByText('Advanced Options')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  // Click to expand - find the button/header element
  const accordionHeader = screen.getByText('Advanced Options')
  const clickableElement =
    accordionHeader.closest('button') ||
    accordionHeader.closest('[role="button"]') ||
    accordionHeader
  fireEvent.click(clickableElement)

  await waitFor(
    () => {
      // There might be multiple elements with "Threads" text, so use getAllByLabelText
      const threadsInputs = screen.getAllByLabelText(/Threads/)
      expect(threadsInputs.length).toBeGreaterThan(0)
    },
    { timeout: 2000 }
  )
})

test('closes config panel when cancel is clicked', async () => {
  const onClose = vi.fn()

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose,
      onSave: vi.fn()
    }
  })

  await waitFor(
    () => {
      expect(screen.getByText('Server Configuration')).toBeTruthy()
    },
    { timeout: 2000 }
  )

  const cancelButton = screen.getByText('Cancel')
  fireEvent.click(cancelButton)

  await waitFor(
    () => {
      expect(onClose).toHaveBeenCalledTimes(1)
    },
    { timeout: 2000 }
  )
})

test('closes config panel when close button is clicked', async () => {
  const onClose = vi.fn()

  render(LlamaConfig as Component, {
    props: {
      isOpen: true,
      onClose,
      onSave: vi.fn()
    }
  })

  const closeButton = screen.getByLabelText('Close')
  fireEvent.click(closeButton)

  expect(onClose).toHaveBeenCalledTimes(1)
})

test('loads the agent debug logging flag and persists the toggled value', async () => {
  mockGets({
    'llama-server/config': { hf_model: 'saved-model', ctx_size: 0 },
    'agent/config': { debug_logging: true }
  })
  mockedAxios.post.mockResolvedValue({
    data: { success: true, message: 'Config saved' }
  })

  await openPanel({ onSave: vi.fn() })

  const debugCheckbox = (await waitFor(() =>
    screen.getByLabelText('Debug Conversation Logging')
  )) as HTMLInputElement
  expect(debugCheckbox).toBeChecked()

  await fireEvent.click(debugCheckbox)
  expect(debugCheckbox).not.toBeChecked()

  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      debug_logging: false
    })
  })
})

test('completes the save even when persisting the debug flag fails', async () => {
  mockGets({
    'llama-server/config': { hf_model: 'saved-model', ctx_size: 0 }
  })
  mockedAxios.post.mockImplementation((url: string) => {
    if (url === 'agent/config') {
      return Promise.reject(new Error('agent config unavailable'))
    }
    return Promise.resolve({ data: { success: true, message: 'ok' } })
  })

  const onClose = vi.fn()
  const onSave = vi.fn()
  await openPanel({ onClose, onSave })

  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(onClose).toHaveBeenCalledTimes(1)
  })
  expect(onSave).toHaveBeenCalledTimes(1)
  expect(console.error).toHaveBeenCalledWith(
    'Failed to save agent config (debug logging):',
    expect.any(Error)
  )
})

test('only applies llama model notes that are keyed by model name', async () => {
  mockGets({
    'llama-server/config': { hf_model: '', ctx_size: 0 },
    'llama-server/models': {
      local_models: [
        { name: 'alpha-model.gguf', path: '/models/alpha-model.gguf' },
        { name: 'beta-model.gguf', path: '/models/beta-model.gguf' }
      ]
    },
    'model-notes': {
      notes: [
        {
          platform: 'llama',
          model_name: 'alpha-model.gguf',
          is_favorite: true,
          is_default: false,
          tags: ['fast'],
          notes: 'my favourite'
        },
        {
          // no model_name -> stored under model_path, but never matched
          platform: 'llama',
          model_name: '',
          model_path: '/models/beta-model.gguf',
          is_favorite: true,
          is_default: false,
          tags: ['by-path']
        },
        {
          // neither name nor path -> dropped entirely
          platform: 'llama',
          model_name: '',
          model_path: '',
          is_favorite: true,
          is_default: false,
          tags: ['dropped']
        },
        {
          // wrong platform -> ignored
          platform: 'ollama',
          model_name: 'alpha-model.gguf',
          is_favorite: true,
          is_default: false,
          tags: ['ollama-only']
        }
      ]
    }
  })

  const { container } = await openPanel({ onSave: vi.fn() })

  await waitFor(() => {
    expect(screen.getByText('fast')).toBeTruthy()
  })
  expect(screen.getByText('my favourite')).toBeTruthy()
  expect(container.querySelectorAll('.favorite-icon')).toHaveLength(1)
  expect(screen.queryByText('by-path')).not.toBeInTheDocument()
  expect(screen.queryByText('dropped')).not.toBeInTheDocument()
  expect(screen.queryByText('ollama-only')).not.toBeInTheDocument()
})

test('shows a fallback error when loading models fails without details', async () => {
  mockGets({ 'llama-server/models': rejectWith({}) })

  await openPanel({ onSave: vi.fn() })

  await waitFor(() => {
    expect(screen.getByText('Failed to load models')).toBeTruthy()
  })
})

test('selecting a model with an hf_format fills the backend value and path', async () => {
  mockGets({
    'llama-server/config': { hf_model: '', ctx_size: 4096 },
    'llama-server/models': {
      local_models: [
        {
          name: 'alpha-model.gguf',
          path: '/models/alpha-model.gguf',
          hf_format: 'org/alpha-model:Q4_K_M'
        }
      ]
    }
  })

  await openPanel({ onSave: vi.fn() })
  await openAdvancedOptions()

  await waitFor(() => {
    expect(screen.getByText('alpha-model.gguf')).toBeTruthy()
  })
  fireEvent.click(screen.getByText('alpha-model.gguf').closest('button')!)

  await waitFor(() => {
    expect(hfModelInput().value).toBe('org/alpha-model:Q4_K_M')
  })
  expect((screen.getByLabelText('Model Path') as HTMLInputElement).value).toBe(
    '/models/alpha-model.gguf'
  )
  // selecting a model resets the context size so llama.cpp picks the default
  expect(
    (screen.getByLabelText('Context Size') as HTMLInputElement).value
  ).toBe('0')
})

test('selecting a model without hf_format or path falls back to its name', async () => {
  mockGets({
    'llama-server/models': {
      local_models: [{ name: 'beta-model.gguf' }]
    }
  })

  await openPanel({ onSave: vi.fn() })
  await openAdvancedOptions()

  await waitFor(() => {
    expect(screen.getByText('beta-model.gguf')).toBeTruthy()
  })
  fireEvent.click(screen.getByText('beta-model.gguf').closest('button')!)

  await waitFor(() => {
    expect(hfModelInput().value).toBe('beta-model.gguf')
  })
  // no path on the model -> the model path field is left alone
  expect((screen.getByLabelText('Model Path') as HTMLInputElement).value).toBe(
    ''
  )
})

test('a manually typed model becomes the backend value and clears the model path', async () => {
  mockGets({
    'llama-server/config': {
      hf_model: '',
      ctx_size: 0,
      model: '/models/previous.gguf'
    }
  })
  mockedAxios.post.mockResolvedValue({
    data: { success: true, message: 'ok' }
  })

  await openPanel({ onSave: vi.fn() })
  await openAdvancedOptions()

  await waitFor(() => {
    expect(
      (screen.getByLabelText('Model Path') as HTMLInputElement).value
    ).toBe('/models/previous.gguf')
  })

  await fireEvent.input(hfModelInput(), {
    target: { value: 'org/manual-model:Q8_0' }
  })

  await waitFor(() => {
    expect(
      (screen.getByLabelText('Model Path') as HTMLInputElement).value
    ).toBe('')
  })

  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('llama-server/config', {
      hf_model: 'org/manual-model:Q8_0',
      ctx_size: 0,
      model: ''
    })
  })
})

test('saves with only a model path when no HuggingFace model is set', async () => {
  mockGets({
    'llama-server/config': {
      hf_model: '',
      ctx_size: 0,
      model: '/models/only-path.gguf'
    }
  })
  mockedAxios.post.mockResolvedValue({
    data: { success: true, message: 'ok' }
  })

  const onClose = vi.fn()
  await openPanel({ onClose, onSave: vi.fn() })

  await waitFor(() => {
    expect(screen.getByText('Save')).not.toBeDisabled()
  })
  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('llama-server/config', {
      hf_model: '',
      ctx_size: 0,
      model: '/models/only-path.gguf'
    })
  })
  await waitFor(() => {
    expect(onClose).toHaveBeenCalledTimes(1)
  })
})

test('rejects an invalid batch size before calling the backend', async () => {
  mockGets({
    'llama-server/config': { hf_model: 'saved-model', ctx_size: 0 }
  })

  await openPanel({ onSave: vi.fn() })
  await openAdvancedOptions()

  await fireEvent.input(screen.getByLabelText('Batch Size'), {
    target: { value: '0' }
  })

  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(screen.getByText('Batch size must be greater than 0')).toBeTruthy()
  })
  expect(mockedAxios.post).not.toHaveBeenCalled()
  // the save button is released again after the validation error
  expect(screen.getByText('Save')).not.toBeDisabled()
})

test('reloads the config and closes when no onSave callback is given', async () => {
  mockGets({
    'llama-server/config': { hf_model: 'saved-model', ctx_size: 0 }
  })
  mockedAxios.post.mockResolvedValue({
    data: { success: true, message: 'ok' }
  })

  const onClose = vi.fn()
  await openPanel({ onClose })

  const configGetsBefore = mockedAxios.get.mock.calls.filter(
    (call: unknown[]) => call[0] === 'llama-server/config'
  ).length

  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(onClose).toHaveBeenCalledTimes(1)
  })
  const configGetsAfter = mockedAxios.get.mock.calls.filter(
    (call: unknown[]) => call[0] === 'llama-server/config'
  ).length
  expect(configGetsAfter).toBe(configGetsBefore + 1)
})

test('surfaces the backend error payload when saving throws', async () => {
  mockGets({
    'llama-server/config': { hf_model: 'saved-model', ctx_size: 0 }
  })
  mockedAxios.post.mockRejectedValueOnce({
    response: { data: { error: 'disk is full' } },
    message: 'Request failed with status code 500'
  })

  await openPanel({ onSave: vi.fn() })
  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(screen.getByText('disk is full')).toBeTruthy()
  })
})

test('falls back to the error message when saving throws without a payload', async () => {
  mockGets({
    'llama-server/config': { hf_model: 'saved-model', ctx_size: 0 }
  })
  mockedAxios.post.mockRejectedValueOnce(new Error('gateway timeout'))

  await openPanel({ onSave: vi.fn() })
  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(screen.getByText('gateway timeout')).toBeTruthy()
  })
})

test('falls back to a generic message when saving throws without details', async () => {
  mockGets({
    'llama-server/config': { hf_model: 'saved-model', ctx_size: 0 }
  })
  mockedAxios.post.mockRejectedValueOnce({})

  const onClose = vi.fn()
  await openPanel({ onClose, onSave: vi.fn() })
  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(screen.getByText('Failed to save config')).toBeTruthy()
  })
  expect(onClose).not.toHaveBeenCalled()
  expect(screen.getByText('Save')).not.toBeDisabled()
})

test('sends every advanced option in the save payload', async () => {
  mockGets({
    'llama-server/config': { hf_model: 'saved-model', ctx_size: 0 }
  })
  mockedAxios.post.mockResolvedValue({
    data: { success: true, message: 'ok' }
  })

  await openPanel({ onSave: vi.fn() })
  await openAdvancedOptions()

  await fireEvent.input(screen.getByLabelText('Threads'), {
    target: { value: '8' }
  })
  await fireEvent.input(screen.getByLabelText('Threads Batch'), {
    target: { value: '4' }
  })
  await fireEvent.input(screen.getByLabelText('Predict (N Predict)'), {
    target: { value: '256' }
  })
  await fireEvent.input(screen.getByLabelText('Batch Size'), {
    target: { value: '512' }
  })
  await fireEvent.input(screen.getByLabelText('UBatch Size'), {
    target: { value: '128' }
  })
  await fireEvent.input(screen.getByLabelText('GPU Layers'), {
    target: { value: '99' }
  })
  await fireEvent.input(screen.getByLabelText('Model Path'), {
    target: { value: '  /models/typed.gguf  ' }
  })
  await fireEvent.click(screen.getByLabelText('Flash Attention'))
  await fireEvent.click(screen.getByLabelText('MLock'))
  await fireEvent.click(screen.getByLabelText('No MMAP'))
  await fireEvent.input(screen.getByLabelText('Context Size'), {
    target: { value: '8192' }
  })

  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('llama-server/config', {
      hf_model: 'saved-model',
      ctx_size: 8192,
      threads: 8,
      threads_batch: 4,
      predict: 256,
      batch_size: 512,
      ubatch_size: 128,
      gpu_layers: 99,
      flash_attn: true,
      mlock: true,
      no_mmap: true,
      model: '/models/typed.gguf'
    })
  })
})

test('shows the saving state while the request is in flight', async () => {
  mockGets({
    'llama-server/config': { hf_model: 'saved-model', ctx_size: 0 }
  })
  mockedAxios.post.mockImplementation(() => new Promise(() => {}))

  await openPanel({ onSave: vi.fn() })
  fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(screen.getByText('Saving...')).toBeDisabled()
  })
})
