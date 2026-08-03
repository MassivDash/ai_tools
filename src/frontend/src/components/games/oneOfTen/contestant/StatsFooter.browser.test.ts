// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen } from '@testing-library/svelte'
import { describe, it, expect } from 'vitest'
import StatsFooter from './StatsFooter.svelte'

describe('StatsFooter', () => {
  it('renders score and one heart per life, without strikes outside round 1', () => {
    render(StatsFooter, {
      props: { score: 150, lives: 3, isRound1: false }
    })

    expect(screen.getByText('150')).toBeInTheDocument()
    expect(screen.getByText('❤️❤️❤️')).toBeInTheDocument()
    expect(screen.queryByText('Strikes')).not.toBeInTheDocument()
  })

  it('renders no hearts for a negative life count', () => {
    render(StatsFooter, {
      props: { score: 0, lives: -1, isRound1: false }
    })

    const value = screen.getByText('Lives').nextElementSibling
    expect(value).toHaveTextContent('')
  })

  it('shows the strike counter in round 1', () => {
    render(StatsFooter, {
      props: { score: 10, lives: 2, isRound1: true, round1Misses: 1 }
    })

    expect(screen.getByText('Strikes')).toBeInTheDocument()
    expect(screen.getByText('1/2')).toBeInTheDocument()
  })

  it('defaults the strike counter to zero when misses are not provided', () => {
    render(StatsFooter, {
      props: { score: 10, lives: 2, isRound1: true }
    })

    expect(screen.getByText('0/2')).toBeInTheDocument()
  })
})
