// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent } from '@testing-library/svelte'
import { describe, it, expect, vi } from 'vitest'
import Round3Decision from './Round3Decision.svelte'
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

describe('Round3Decision', () => {
  it('offers the two decisions before a target is being picked', () => {
    render(Round3Decision, {
      props: { players, myId: 'me', onMakeDecision: vi.fn() }
    })

    expect(
      screen.getByText('Correct! What do you want to do?')
    ).toBeInTheDocument()
    expect(screen.getByText('Double Down (Self)')).toBeInTheDocument()
    expect(screen.getByText('Point to Player')).toBeInTheDocument()
    expect(
      screen.queryByText('Select a player to answer:')
    ).not.toBeInTheDocument()
  })

  it('reports a self decision with no target', async () => {
    const onMakeDecision = vi.fn()
    render(Round3Decision, { props: { players, myId: 'me', onMakeDecision } })

    await fireEvent.click(screen.getByText('Double Down (Self)'))

    expect(onMakeDecision).toHaveBeenCalledTimes(1)
    expect(onMakeDecision).toHaveBeenCalledWith('self')
  })

  it('shows the player grid excluding self after choosing to point', async () => {
    const onMakeDecision = vi.fn()
    render(Round3Decision, { props: { players, myId: 'me', onMakeDecision } })

    await fireEvent.click(screen.getByText('Point to Player'))

    expect(screen.getByText('Select a player to answer:')).toBeInTheDocument()
    expect(screen.getByText('Rival')).toBeInTheDocument()
    expect(screen.queryByText('Me')).not.toBeInTheDocument()
    expect(screen.queryByText('Double Down (Self)')).not.toBeInTheDocument()
    expect(onMakeDecision).not.toHaveBeenCalled()
  })

  it('reports a point decision with the selected target id', async () => {
    const onMakeDecision = vi.fn()
    render(Round3Decision, { props: { players, myId: 'me', onMakeDecision } })

    await fireEvent.click(screen.getByText('Point to Player'))
    await fireEvent.click(screen.getByText('Rival'))

    expect(onMakeDecision).toHaveBeenCalledTimes(1)
    expect(onMakeDecision).toHaveBeenCalledWith('point', 'rival')
  })

  it('goes back to the decision buttons from the grid', async () => {
    render(Round3Decision, {
      props: { players, myId: 'me', onMakeDecision: vi.fn() }
    })

    await fireEvent.click(screen.getByText('Point to Player'))
    await fireEvent.click(screen.getByText('Back'))

    expect(screen.getByText('Double Down (Self)')).toBeInTheDocument()
    expect(
      screen.queryByText('Select a player to answer:')
    ).not.toBeInTheDocument()
  })
})
