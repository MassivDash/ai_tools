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
        data: { hf_model: '', ctx_size: 10240 }
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
        data: { hf_model: '', ctx_size: 10240 }
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
        data: { hf_model: '', ctx_size: 10240 }
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
        data: { hf_model: '', ctx_size: 10240 }
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
        data: { hf_model: '', ctx_size: 10240 }
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
    data: { hf_model: 'test-model', ctx_size: 10240 }
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
        data: { hf_model: '', ctx_size: 10240 }
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
        data: { hf_model: '', ctx_size: 10240 }
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
        data: { hf_model: 'test-model', ctx_size: 10240 }
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
      expect(ctxInput.value).toBe('10240')
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
        data: { hf_model: 'test-model', ctx_size: 10240 }
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
