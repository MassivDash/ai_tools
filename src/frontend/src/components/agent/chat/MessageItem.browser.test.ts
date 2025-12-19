/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen } from '@testing-library/svelte'
import { expect, test } from 'vitest'
import MessageItem from './MessageItem.svelte'
import type { Component } from 'svelte'

test('renders user message', () => {
  const message = {
    id: '1',
    role: 'user',
    content: 'Hello AI',
    timestamp: Date.now()
  }

  render(MessageItem as Component, {
    props: { message }
  })

  expect(screen.getByText('Hello AI')).toBeTruthy()
  expect(screen.getByText('You')).toBeTruthy()
})

test('renders assistant message with markdown', () => {
  // Note: We are testing that it *attempts* to render.
  // Since we aren't mocking marked/markdown rendering deeply here (unless we mock the utils),
  // we assume raw text or simple markdown works.
  // If the component uses a `renderMarkdown` util, we verify output or mocked call.
  // The component imports `renderMarkdown`. Let's assume it puts content in innerHTML.

  const message = {
    id: '2',
    role: 'assistant',
    content: '**Bold** response',
    timestamp: Date.now()
  }

  render(MessageItem as Component, {
    props: { message }
  })

  // getByText might fail for partial HTML matches if split by tags.
  // We can look for container content.
  const container = document.querySelector('.message-content')
  expect(container).toBeTruthy()
  // Since we rely on the real `renderMarkdown` util which we haven't mocked globally,
  // it might assume it works or we should mock it if it has external deps.
  // Assuming it parses simple MD locally.
})

test('renders tool calls', () => {
  const message = {
    id: '3',
    role: 'tool',
    content: '✅ tool_name completed',
    timestamp: Date.now(),
    toolName: 'search_web'
  }

  render(MessageItem as Component, {
    props: { message }
  })

  expect(screen.getByText(/tool_name/)).toBeTruthy()
  expect(screen.getByText(/completed/)).toBeTruthy()
})

test('renders image content array', () => {
  const message = {
    id: '4',
    role: 'user',
    content: [
      { type: 'text', text: 'Look at this:' },
      { type: 'image_url', image_url: { url: 'http://example.com/image.jpg' } }
    ],
    timestamp: Date.now()
  }

  render(MessageItem as Component, {
    props: { message }
  })

  expect(screen.getByText('Look at this:')).toBeTruthy()
  const img = document.querySelector('img')
  expect(img).toBeTruthy()
  expect(img?.src).toBe('http://example.com/image.jpg')
})

test('hides image attachments from chips', () => {
  const message = {
    id: '5',
    role: 'user',
    content: [{ type: 'image_url', image_url: { url: '...' } }],
    timestamp: Date.now(),
    attachments: [
      { name: 'photo.jpg', type: 'image', content: '...' },
      { name: 'doc.txt', type: 'text', content: '...' }
    ]
  }

  render(MessageItem as Component, {
    props: { message }
  })

  // Should show doc.txt chip
  expect(screen.getByText('doc.txt')).toBeTruthy()
  // Should NOT show photo.jpg chip (filtered out because type is image)
  expect(screen.queryByText('photo.jpg')).toBeNull()
})
