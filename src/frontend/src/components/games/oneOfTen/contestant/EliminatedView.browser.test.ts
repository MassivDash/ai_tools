// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen } from '@testing-library/svelte'
import { describe, it, expect } from 'vitest'
import EliminatedView from './EliminatedView.svelte'

describe('EliminatedView', () => {
  it('shows the elimination notice and the final score', () => {
    render(EliminatedView, { props: { score: 420, isRound3: false } })

    expect(screen.getByText('ELIMINATED')).toBeInTheDocument()
    expect(
      screen.getByText('You have been eliminated from the game.')
    ).toBeInTheDocument()
    expect(screen.getByText('Final Score: 420')).toBeInTheDocument()
    expect(
      screen.queryByText('Current Round: The Buzzer!')
    ).not.toBeInTheDocument()
  })

  it('mentions the buzzer round while round 3 is running', () => {
    render(EliminatedView, { props: { score: 0, isRound3: true } })

    expect(screen.getByText('Current Round: The Buzzer!')).toBeInTheDocument()
    expect(screen.getByText('Final Score: 0')).toBeInTheDocument()
  })
})
