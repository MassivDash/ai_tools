/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import ChromaDBConfigSection from './ChromaDBConfigSection.svelte'
import { axiosBackendInstance } from '../../../axiosInstance/axiosBackendInstance.ts'
import type { Component } from 'svelte'

// Mock axiosBackendInstance
vi.mock('../../../axiosInstance/axiosBackendInstance.ts', () => ({
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
  vi.spyOn(console, 'log').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

const defaultProps = {
  chromadbEnabled: false,
  collections: [],
  models: [],
  selectedCollection: '',
  selectedEmbeddingModel: '',
  loadingCollections: false,
  loadingModels: false,
  onToggle: vi.fn(),
  onCollectionSelect: vi.fn(),
  onModelSelect: vi.fn()
}

test('renders disabled state correctly', () => {
  render(ChromaDBConfigSection as Component, { props: defaultProps })
  expect(screen.getByText('Knowledge Base')).toBeTruthy()
  expect(screen.getByText('Enable ChromaDB')).toBeTruthy()
  // Content should not be visible
  expect(screen.queryByText('Collection')).toBeNull()
})

test('renders enabled state with empty lists', async () => {
  render(ChromaDBConfigSection as Component, {
    props: { ...defaultProps, chromadbEnabled: true }
  })

  // Wait for model notes load
  await waitFor(() => {
    expect(mockedAxios.get).toHaveBeenCalledWith('model-notes')
  })

  expect(screen.getByText('Collection')).toBeTruthy()
  expect(screen.getByText('Embedding Model')).toBeTruthy()
  expect(screen.getByText('No collections found')).toBeTruthy()
  expect(screen.getByText('No Ollama models found')).toBeTruthy()
})

test('renders collections and models', async () => {
  const collections = [{ id: 'c1', name: 'My Collection', count: 10 }]
  const models = [{ name: 'nomic-embed-text', size: '2GB' }]

  render(ChromaDBConfigSection as Component, {
    props: {
      ...defaultProps,
      chromadbEnabled: true,
      collections,
      models
    }
  })

  expect(screen.getByText('My Collection')).toBeTruthy()
  expect(screen.getByText('10 documents')).toBeTruthy()
  expect(screen.getByText('nomic-embed-text')).toBeTruthy()
})

test('handles toggling', async () => {
  const onToggle = vi.fn()
  render(ChromaDBConfigSection as Component, {
    props: { ...defaultProps, onToggle }
  })

  const input = screen.getByLabelText('Enable ChromaDB')
  await fireEvent.click(input)

  expect(onToggle).toHaveBeenCalled()
})
