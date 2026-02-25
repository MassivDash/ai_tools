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
})
