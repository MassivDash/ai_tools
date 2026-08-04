// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent } from '@testing-library/svelte'
import { describe, it, expect, vi } from 'vitest'
import LobbyView from './LobbyView.svelte'

describe('LobbyView', () => {
  it('prompts an unready contestant to declare readiness', async () => {
    const onToggleReady = vi.fn()
    render(LobbyView, { props: { isReady: false, onToggleReady } })

    expect(screen.getByText('Are you ready to play?')).toBeInTheDocument()
    expect(
      screen.getByText('Click the button below when you are ready.')
    ).toBeInTheDocument()
    expect(screen.queryByText('You are Ready!')).not.toBeInTheDocument()

    const readyBtn = screen.getByRole('button')
    expect(readyBtn).toHaveTextContent("I'M READY!")
    expect(readyBtn).toHaveClass('btn-ready')

    await fireEvent.click(readyBtn)
    expect(onToggleReady).toHaveBeenCalledTimes(1)
  })

  it('confirms readiness and offers to cancel it', async () => {
    const onToggleReady = vi.fn()
    render(LobbyView, { props: { isReady: true, onToggleReady } })

    expect(screen.getByText('You are Ready!')).toBeInTheDocument()
    expect(
      screen.getByText('Waiting for the game to start...')
    ).toBeInTheDocument()
    expect(screen.queryByText('Are you ready to play?')).not.toBeInTheDocument()

    const cancelBtn = screen.getByRole('button')
    expect(cancelBtn).toHaveTextContent('Cancel Ready')
    expect(cancelBtn).toHaveClass('btn-not-ready')

    await fireEvent.click(cancelBtn)
    expect(onToggleReady).toHaveBeenCalledTimes(1)
  })

  it('swaps views when readiness changes', async () => {
    const { rerender } = render(LobbyView, {
      props: { isReady: false, onToggleReady: vi.fn() }
    })

    expect(screen.getByText('Are you ready to play?')).toBeInTheDocument()

    await rerender({ isReady: true, onToggleReady: vi.fn() })

    expect(screen.getByText('You are Ready!')).toBeInTheDocument()
    expect(screen.queryByText('Are you ready to play?')).not.toBeInTheDocument()
  })
})
