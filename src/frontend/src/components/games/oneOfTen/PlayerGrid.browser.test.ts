// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent } from '@testing-library/svelte'
import { describe, it, expect, vi } from 'vitest'
import PlayerGrid from './PlayerGrid.svelte'
import type { Contestant } from '../../../hooks/useOneOfTenState.svelte'

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

describe('PlayerGrid', () => {
  it('renders one card per eligible player with name, age and lives', () => {
    render(PlayerGrid, {
      props: {
        players: [
          makeContestant({ id: 'c1', name: 'Alice', age: '30', lives: 3 }),
          makeContestant({ id: 'c2', name: 'Bob', age: '41', lives: 1 })
        ],
        onSelect: vi.fn()
      }
    })

    const cards = screen.getAllByRole('button')
    expect(cards).toHaveLength(2)
    expect(cards[0]).toHaveTextContent('Alice')
    expect(cards[0]).toHaveTextContent('Age: 30')
    expect(cards[0]).toHaveTextContent('Lives: 3')
    expect(cards[1]).toHaveTextContent('Bob')
    expect(cards[1]).toHaveTextContent('Age: 41')
    expect(cards[1]).toHaveTextContent('Lives: 1')
    expect(
      screen.queryByText('No eligible players to point to.')
    ).not.toBeInTheDocument()
  })

  it('calls onSelect with the clicked player id', async () => {
    const onSelect = vi.fn()
    render(PlayerGrid, {
      props: {
        players: [
          makeContestant({ id: 'c1', name: 'Alice' }),
          makeContestant({ id: 'c2', name: 'Bob' })
        ],
        onSelect
      }
    })

    await fireEvent.click(screen.getByText('Bob'))

    expect(onSelect).toHaveBeenCalledTimes(1)
    expect(onSelect).toHaveBeenCalledWith('c2')
  })

  it('excludes eliminated players and the excludeId player', () => {
    render(PlayerGrid, {
      props: {
        players: [
          makeContestant({ id: 'me', name: 'Me' }),
          makeContestant({ id: 'gone', name: 'Gone', eliminated: true }),
          makeContestant({ id: 'other', name: 'Other' })
        ],
        onSelect: vi.fn(),
        excludeId: 'me'
      }
    })

    expect(screen.getAllByRole('button')).toHaveLength(1)
    expect(screen.getByText('Other')).toBeInTheDocument()
    expect(screen.queryByText('Me')).not.toBeInTheDocument()
    expect(screen.queryByText('Gone')).not.toBeInTheDocument()
  })

  it('shows the empty message when nobody is eligible', () => {
    render(PlayerGrid, {
      props: {
        players: [
          makeContestant({ id: 'me', name: 'Me' }),
          makeContestant({ id: 'gone', name: 'Gone', eliminated: true })
        ],
        onSelect: vi.fn(),
        excludeId: 'me'
      }
    })

    expect(
      screen.getByText('No eligible players to point to.')
    ).toBeInTheDocument()
    expect(screen.queryAllByRole('button')).toHaveLength(0)
  })

  it('disables every card when disabled is set', () => {
    render(PlayerGrid, {
      props: {
        players: [
          makeContestant({ id: 'c1', name: 'Alice' }),
          makeContestant({ id: 'c2', name: 'Bob' })
        ],
        onSelect: vi.fn(),
        disabled: true
      }
    })

    for (const card of screen.getAllByRole('button')) {
      expect(card).toBeDisabled()
    }
  })

  it('leaves cards enabled by default', () => {
    render(PlayerGrid, {
      props: {
        players: [makeContestant({ id: 'c1', name: 'Alice' })],
        onSelect: vi.fn()
      }
    })

    expect(screen.getByRole('button')).toBeEnabled()
  })
})
