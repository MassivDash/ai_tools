// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen } from '@testing-library/svelte'
import { describe, it, expect } from 'vitest'
import PresenterContestantList from './PresenterContestantList.svelte'
import type { Contestant } from '../../../../hooks/useOneOfTenState.svelte'

function makeContestant(overrides: Partial<Contestant> = {}): Contestant {
  return {
    id: 'c1',
    name: 'Alice',
    age: '30',
    score: 0,
    lives: 3,
    round1_misses: 0,
    round1_questions: 0,
    online: true,
    eliminated: false,
    ready: false,
    session_id: 's1',
    ...overrides
  }
}

describe('PresenterContestantList', () => {
  it('counts the contestants and shows their age, lives and score', () => {
    const { container } = render(PresenterContestantList, {
      props: {
        contestants: [
          makeContestant({
            id: 'c1',
            name: 'Alice',
            age: '30',
            lives: 2,
            score: 40
          }),
          makeContestant({
            id: 'c2',
            name: 'Bob',
            age: '41',
            lives: 3,
            score: 0
          })
        ],
        round: 'round2'
      }
    })

    expect(screen.getByText('Contestants (2)')).toBeInTheDocument()
    expect(screen.getByText('(30)')).toBeInTheDocument()
    expect(screen.getByText('(41)')).toBeInTheDocument()
    expect(screen.getByText('❤️ 2')).toBeInTheDocument()
    expect(screen.getByText('⭐ 40')).toBeInTheDocument()
    // Miss counters are round 1 only
    expect(container.querySelector('[title="Misses"]')).toBeNull()
  })

  it('omits the age when it is not known', () => {
    render(PresenterContestantList, {
      props: {
        contestants: [makeContestant({ age: '' })],
        round: 'lobby'
      }
    })

    expect(screen.getByText('Alice')).toBeInTheDocument()
    expect(screen.queryByText('()')).not.toBeInTheDocument()
  })

  it('shows the ready badge only in the lobby', () => {
    const { unmount } = render(PresenterContestantList, {
      props: {
        contestants: [
          makeContestant({ id: 'c1', name: 'Alice', ready: true }),
          makeContestant({ id: 'c2', name: 'Bob', ready: false })
        ],
        round: 'lobby'
      }
    })

    expect(screen.getAllByText('READY')).toHaveLength(1)
    unmount()

    render(PresenterContestantList, {
      props: {
        contestants: [makeContestant({ ready: true })],
        round: 'round1'
      }
    })

    expect(screen.queryByText('READY')).not.toBeInTheDocument()
  })

  it('flags eliminated contestants instead of their ready state', () => {
    const { container } = render(PresenterContestantList, {
      props: {
        contestants: [makeContestant({ eliminated: true, ready: true })],
        round: 'lobby'
      }
    })

    expect(screen.getByText('ELIMINATED')).toBeInTheDocument()
    expect(screen.queryByText('READY')).not.toBeInTheDocument()
    expect(container.querySelector('li')).toHaveClass('eliminated')
  })

  it('marks offline contestants', () => {
    const { container } = render(PresenterContestantList, {
      props: {
        contestants: [makeContestant({ online: false })],
        round: 'round1'
      }
    })

    const row = container.querySelector('li')
    expect(row).toHaveClass('offline')
    expect(row).not.toHaveClass('online')
  })

  it('shows the miss counter during round 1', () => {
    render(PresenterContestantList, {
      props: {
        contestants: [makeContestant({ round1_misses: 1 })],
        round: 'round1'
      }
    })

    expect(screen.getByTitle('Misses')).toHaveTextContent('❌ 1/2')
  })

  it('renders an empty list when nobody has joined', () => {
    const { container } = render(PresenterContestantList, {
      props: { contestants: [], round: 'lobby' }
    })

    expect(screen.getByText('Contestants (0)')).toBeInTheDocument()
    expect(container.querySelectorAll('li')).toHaveLength(0)
  })
})
