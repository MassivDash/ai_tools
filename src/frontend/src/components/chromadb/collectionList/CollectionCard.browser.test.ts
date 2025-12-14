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

test('does not render metadata section when metadata is empty', () => {
  const { container } = render(CollectionCard, {
    props: { collection: mockCollection }
  })

  const metadataSection = container.querySelector('.metadata')
  expect(metadataSection).not.toBeInTheDocument()
})
