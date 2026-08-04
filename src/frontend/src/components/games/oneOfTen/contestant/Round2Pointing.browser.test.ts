// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent } from '@testing-library/svelte'
import { describe, it, expect, vi } from 'vitest'
import Round2Pointing from './Round2Pointing.svelte'
import type { Contestant } from '../../../../hooks/useOneOfTenState.svelte'

function makeContestant(overrides: Partial<Contestant> = {}): Contestant {
  return {
    id: 'c1',
    name: 'Player 1',
    age: '25',
    score: 0,
    session_id: 's1',
    online: true,
    ready: false,
    lives: 3,
    round1_misses: 0,
    round1_questions: 0,
    eliminated: false,
    ...overrides
  }
}

const players = [
  makeContestant({ id: 'me', name: 'Me' }),
  makeContestant({ id: 'rival', name: 'Rival' })
]

describe('Round2Pointing', () => {
  it('lets the pointing player choose someone other than themselves', async () => {
    const onPointToPlayer = vi.fn()
    render(Round2Pointing, {
      props: {
        isMyTurnToPoint: true,
        players,
        myId: 'me',
        pointerName: 'Me',
        onPointToPlayer
      }
    })

    expect(
      screen.getByText("It's your turn to choose the next player!")
    ).toBeInTheDocument()
    expect(screen.queryByText('Me')).not.toBeInTheDocument()

    await fireEvent.click(screen.getByText('Rival'))

    expect(onPointToPlayer).toHaveBeenCalledTimes(1)
    expect(onPointToPlayer).toHaveBeenCalledWith('rival')
  })

  it('shows a spectator message naming the pointer for everyone else', () => {
    const onPointToPlayer = vi.fn()
    render(Round2Pointing, {
      props: {
        isMyTurnToPoint: false,
        players,
        myId: 'rival',
        pointerName: 'Me',
        onPointToPlayer
      }
    })

    expect(screen.getByText('Pointing Phase')).toBeInTheDocument()
    expect(
      screen.getByText('Waiting for Me to select a player...')
    ).toBeInTheDocument()
    expect(
      screen.queryByText("It's your turn to choose the next player!")
    ).not.toBeInTheDocument()
    expect(screen.queryAllByRole('button')).toHaveLength(0)
    expect(onPointToPlayer).not.toHaveBeenCalled()
  })
})
