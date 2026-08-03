/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import AgentHeader from './AgentHeader.svelte'
import type { Component } from 'svelte'

test('AgentHeader renders and buttons are clickable', async () => {
  const onToggleConfig = vi.fn()
  const onToggleLlamaConfig = vi.fn()
  const onToggleTerminal = vi.fn()
  const onToggleHistory = vi.fn()
  const onToggleTesting = vi.fn()
  const onNewChat = vi.fn()

  const props = {
    showConfig: false,
    showLlamaConfig: false,
    showTerminal: false,
    showHistory: false,
    showTesting: false,
    onToggleConfig,
    onToggleLlamaConfig,
    onToggleTerminal,
    onToggleHistory,
    onToggleTesting,
    onNewChat
  }

  render(AgentHeader as Component, { props })

  // Check Agent Config button
  const configBtn = screen.getByTitle('Agent Config')
  await fireEvent.click(configBtn)
  expect(onToggleConfig).toHaveBeenCalled()

  // Check History button
  const historyBtn = screen.getByTitle('Show History')
  await fireEvent.click(historyBtn)
  expect(onToggleHistory).toHaveBeenCalled()

  // Check New Chat button
  const newChatBtn = screen.getByTitle('New Conversation')
  await fireEvent.click(newChatBtn)
  expect(onNewChat).toHaveBeenCalled()

  // Terminal / Llama config / Testing buttons
  await fireEvent.click(screen.getByTitle('Llama Server Config'))
  expect(onToggleLlamaConfig).toHaveBeenCalled()

  await fireEvent.click(screen.getByTitle('Show Terminal'))
  expect(onToggleTerminal).toHaveBeenCalled()

  await fireEvent.click(screen.getByTitle('Show Testing'))
  expect(onToggleTesting).toHaveBeenCalled()
})

test('AgentHeader button titles flip to the hide wording when panels are open', async () => {
  render(AgentHeader as Component, {
    props: {
      showTerminal: true,
      showHistory: true,
      showTesting: true,
      onToggleConfig: vi.fn(),
      onToggleLlamaConfig: vi.fn(),
      onToggleTerminal: vi.fn(),
      onToggleHistory: vi.fn(),
      onToggleTesting: vi.fn()
    }
  })

  expect(screen.getByTitle('Hide History')).toBeTruthy()
  expect(screen.getByTitle('Hide Terminal')).toBeTruthy()
  expect(screen.getByTitle('Hide Testing')).toBeTruthy()
  expect(screen.queryByTitle('Show History')).toBeNull()
  expect(screen.queryByTitle('Show Terminal')).toBeNull()
  expect(screen.queryByTitle('Show Testing')).toBeNull()
})

test('AgentHeader tolerates a missing onNewChat callback', async () => {
  render(AgentHeader as Component, {
    props: {
      onToggleConfig: vi.fn(),
      onToggleLlamaConfig: vi.fn(),
      onToggleTerminal: vi.fn(),
      onToggleHistory: vi.fn(),
      onToggleTesting: vi.fn()
    }
  })

  // Default noop prop — clicking must not throw, and the header stays rendered
  await fireEvent.click(screen.getByTitle('New Conversation'))
  expect(screen.getByTitle('Show History')).toBeTruthy()
})
