/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import ServerControls from './ServerControls.svelte'
import type { Component } from 'svelte'

const defaultProps = {
  serverActive: false,
  loading: false,
  onStart: vi.fn(),
  onStop: vi.fn()
}

test('renders start button when inactive', () => {
  render(ServerControls as Component, { props: defaultProps })
  const btn = screen.getByRole('button') // Button component renders a button
  // MaterialIcon "play" is used. Title is "Start Server"
  expect(btn).toHaveAttribute('title', 'Start Server')
  expect(btn).not.toBeDisabled()
})

test('renders stop button when active', () => {
  render(ServerControls as Component, {
    props: { ...defaultProps, serverActive: true }
  })
  const btn = screen.getByRole('button')
  // MaterialIcon "stop-circle" is used. Title is "Stop Server"
  expect(btn).toHaveAttribute('title', 'Stop Server')
  expect(btn).not.toBeDisabled()
})

test('calls onStart when start clicked', async () => {
  const onStart = vi.fn()
  render(ServerControls as Component, {
    props: { ...defaultProps, onStart }
  })

  const btn = screen.getByRole('button')
  await fireEvent.click(btn)
  expect(onStart).toHaveBeenCalled()
})

test('calls onStop when stop clicked', async () => {
  const onStop = vi.fn()
  render(ServerControls as Component, {
    props: { ...defaultProps, serverActive: true, onStop }
  })

  const btn = screen.getByRole('button')
  await fireEvent.click(btn)
  expect(onStop).toHaveBeenCalled()
})

test('handles loading state', () => {
  render(ServerControls as Component, {
    props: { ...defaultProps, loading: true }
  })
  const btn = screen.getByRole('button')
  expect(btn).toBeDisabled()
  expect(btn).toHaveAttribute('title', 'Starting...')
})

test('handles stopping loading state', () => {
  render(ServerControls as Component, {
    props: { ...defaultProps, serverActive: true, loading: true }
  })
  const btn = screen.getByRole('button')
  expect(btn).toBeDisabled()
  expect(btn).toHaveAttribute('title', 'Stopping...')
})
