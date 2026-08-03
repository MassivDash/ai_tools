// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen } from '@testing-library/svelte'
import { describe, it, expect } from 'vitest'
import ContestantHeader from './ContestantHeader.svelte'

const baseProps = {
  contestantName: 'Alice',
  statusMessage: 'YOUR TURN!',
  isActivePlayer: false,
  isEliminated: false,
  hasPresenter: false,
  presenterOnline: false
}

describe('ContestantHeader', () => {
  it('greets the contestant and renders the status message', () => {
    render(ContestantHeader, { props: baseProps })

    expect(screen.getByText('Welcome, Alice!')).toBeInTheDocument()
    const badge = screen.getByText('YOUR TURN!')
    expect(badge).toHaveClass('status-badge')
    expect(badge).not.toHaveClass('active')
    expect(badge).not.toHaveClass('eliminated')
  })

  it('falls back to Player when the name is empty', () => {
    render(ContestantHeader, { props: { ...baseProps, contestantName: '' } })

    expect(screen.getByText('Welcome, Player!')).toBeInTheDocument()
  })

  it('marks the badge active for the active player', () => {
    render(ContestantHeader, { props: { ...baseProps, isActivePlayer: true } })

    expect(screen.getByText('YOUR TURN!')).toHaveClass('active')
  })

  it('marks the badge eliminated for an eliminated contestant', () => {
    render(ContestantHeader, {
      props: {
        ...baseProps,
        isEliminated: true,
        statusMessage: 'ELIMINATED'
      }
    })

    expect(screen.getByText('ELIMINATED')).toHaveClass('eliminated')
  })

  it('reports the presenter as offline when there is no presenter', () => {
    render(ContestantHeader, { props: baseProps })

    const status = screen.getByText(/Presenter Offline/)
    expect(status).toHaveClass('presenter-status', 'offline')
    expect(screen.queryByText(/Presenter Online/)).not.toBeInTheDocument()
  })

  it('reports an online presenter when one is connected', () => {
    render(ContestantHeader, {
      props: { ...baseProps, hasPresenter: true, presenterOnline: true }
    })

    const status = screen.getByText(/Presenter Online/)
    expect(status).toHaveClass('presenter-status', 'online')
    expect(status).not.toHaveClass('offline')
  })

  it('reports a disconnected presenter as offline', () => {
    render(ContestantHeader, {
      props: { ...baseProps, hasPresenter: true, presenterOnline: false }
    })

    const status = screen.getByText(/Presenter Offline/)
    expect(status).toHaveClass('presenter-status', 'offline')
    expect(status).not.toHaveClass('online')
  })
})
