/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import CollectionCard from './CollectionCard.svelte'
import type { ChromaDBCollection } from '@types/chromadb.ts'
import type { Component } from 'svelte'

const mockCollection: ChromaDBCollection = {
  id: 'test-collection-id',
  name: 'Test Collection',
  count: 42
}

test('renders collection card with basic info', () => {
  render(CollectionCard as Component, {
    props: { collection: mockCollection }
  })

  expect(screen.getByText('Test Collection')).toBeTruthy()
  expect(screen.getByText('test-collection-id')).toBeTruthy()
  expect(screen.getByText('42')).toBeTruthy()
})

test('dispatches select event when card is clicked', () => {
  const handleSelect = vi.fn()
  render(CollectionCard as Component, {
    props: { collection: mockCollection },
    events: { select: handleSelect }
  })

  const card = screen.getByText('Test Collection').closest('.collection-card')
  fireEvent.click(card!)

  expect(handleSelect).toHaveBeenCalledTimes(1)
})

test('dispatches delete event when delete button is clicked', () => {
  const handleDelete = vi.fn()
  render(CollectionCard as Component, {
    props: { collection: mockCollection },
    events: { delete: handleDelete }
  })

  const deleteButton = screen.getByTitle('Delete collection')
  fireEvent.click(deleteButton)

  expect(handleDelete).toHaveBeenCalledTimes(1)
})

test('does not dispatch select when delete button is clicked', () => {
  const handleSelect = vi.fn()
  render(CollectionCard as Component, {
    props: { collection: mockCollection },
    events: { select: handleSelect }
  })

  const deleteButton = screen.getByTitle('Delete collection')
  fireEvent.click(deleteButton)

  expect(handleSelect).not.toHaveBeenCalled()
})

test('applies selected class when selected prop is true', () => {
  const { container } = render(CollectionCard as Component, {
    props: { collection: mockCollection, selected: true }
  })

  const card = container.querySelector('.collection-card')
  expect(card).toHaveClass('selected')
})

test('does not apply selected class when selected prop is false', () => {
  const { container } = render(CollectionCard as Component, {
    props: { collection: mockCollection, selected: false }
  })

  const card = container.querySelector('.collection-card')
  expect(card).not.toHaveClass('selected')
})

test('renders collection without count', () => {
  const collectionWithoutCount: ChromaDBCollection = {
    id: 'test-id',
    name: 'Test'
  }

  render(CollectionCard as Component, {
    props: { collection: collectionWithoutCount }
  })

  expect(screen.getByText('Test')).toBeTruthy()
  expect(screen.queryByText(/Documents:/)).not.toBeInTheDocument()
})

test('renders collection with metadata', () => {
  const collectionWithMetadata: ChromaDBCollection = {
    id: 'test-id',
    name: 'Test',
    metadata: {
      description: 'Test description',
      category: 'test'
    }
  }

  render(CollectionCard, {
    props: { collection: collectionWithMetadata }
  })

  expect(screen.getByText('description:')).toBeTruthy()
  expect(screen.getByText('Test description')).toBeTruthy()
  expect(screen.getByText('category:')).toBeTruthy()
  expect(screen.getByText('test')).toBeTruthy()
})

test('dispatches select on Enter and Space keydown', () => {
  const handleSelect = vi.fn()
  const { container } = render(CollectionCard as Component, {
    props: { collection: mockCollection },
    events: { select: handleSelect }
  })

  const card = container.querySelector('.collection-card') as HTMLElement

  const enter = new KeyboardEvent('keydown', {
    key: 'Enter',
    bubbles: true,
    cancelable: true
  })
  card.dispatchEvent(enter)
  expect(handleSelect).toHaveBeenCalledTimes(1)
  expect(enter.defaultPrevented).toBe(true)

  const space = new KeyboardEvent('keydown', {
    key: ' ',
    bubbles: true,
    cancelable: true
  })
  card.dispatchEvent(space)
  expect(handleSelect).toHaveBeenCalledTimes(2)
  expect(space.defaultPrevented).toBe(true)
})

test('ignores other keys on keydown', () => {
  const handleSelect = vi.fn()
  const { container } = render(CollectionCard as Component, {
    props: { collection: mockCollection },
    events: { select: handleSelect }
  })

  const card = container.querySelector('.collection-card') as HTMLElement
  const tab = new KeyboardEvent('keydown', {
    key: 'Tab',
    bubbles: true,
    cancelable: true
  })
  card.dispatchEvent(tab)

  expect(handleSelect).not.toHaveBeenCalled()
  expect(tab.defaultPrevented).toBe(false)
})

test('renders the embedding model separately from the other metadata', () => {
  const { container } = render(CollectionCard as Component, {
    props: {
      collection: {
        id: 'test-id',
        name: 'Test',
        metadata: {
          embedding_model: 'nomic-embed-text',
          owner: 'research'
        }
      } as ChromaDBCollection
    }
  })

  expect(screen.getByText('Model:')).toBeTruthy()
  expect(screen.getByText('nomic-embed-text')).toBeTruthy()

  // embedding_model is excluded from the generic metadata list
  const metadataItems = Array.from(
    container.querySelectorAll('.metadata-item')
  ).map((el) => el.textContent?.replace(/\s+/g, ' ').trim())
  expect(metadataItems).toEqual(['owner: research'])
})

test('does not render the metadata list when embedding_model is the only entry', () => {
  const { container } = render(CollectionCard as Component, {
    props: {
      collection: {
        id: 'test-id',
        name: 'Test',
        metadata: { embedding_model: 'nomic-embed-text' }
      } as ChromaDBCollection
    }
  })

  expect(screen.getByText('Model:')).toBeTruthy()
  expect(container.querySelector('.metadata')).toBeNull()
})

test('does not render metadata section when metadata is empty', () => {
  const { container } = render(CollectionCard, {
    props: { collection: mockCollection }
  })

  const metadataSection = container.querySelector('.metadata')
  expect(metadataSection).not.toBeInTheDocument()
})
