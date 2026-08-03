// @vitest-environment jsdom
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import ContestantScreen from './ContestantScreen.svelte'
import type {
  Contestant,
  GameStateSnapshot,
  Question
} from '../../../hooks/useOneOfTenState.svelte'
import '@testing-library/jest-dom' // Fixes toBeInTheDocument

// Mock dependencies

// Lets a single test force a specific phase / flag combination that the real
// hook would derive from a much larger websocket snapshot.
const logicMock = vi.hoisted(() => ({
  overrides: {} as Record<string, unknown>
}))

vi.mock('../../../hooks/useContestantLogic.svelte', () => {
  const PHASE = {
    LOBBY: 'LOBBY',
    ELIMINATED: 'ELIMINATED',
    POINTING: 'POINTING',
    BUZZER: 'BUZZER',
    DECISION: 'DECISION',
    SPECTATING_DECISION: 'SPECTATING_DECISION',
    ANSWERING: 'ANSWERING',
    WAITING: 'WAITING',
    WAITING_FOR_PRESENTER: 'WAITING_FOR_PRESENTER',
    FINISHED: 'FINISHED'
  }
  return {
    PHASE,
    useContestantLogic: (
      getGameState: () => GameStateSnapshot,
      sessionId: string
    ) => {
      // Logic Mock
      const gameState = getGameState()
      const myContestant = gameState.contestants.find((c) => c.id === sessionId)

      let currentPhase: string = PHASE.LOBBY
      if (gameState.round === 'finished') {
        currentPhase = PHASE.FINISHED
      } else if (gameState.round === 'round1') {
        currentPhase =
          myContestant?.id === gameState.active_player_id
            ? PHASE.ANSWERING
            : PHASE.WAITING
      }
      return {
        myContestant,
        isReady: myContestant?.ready || false,
        isActivePlayer: myContestant?.id === gameState.active_player_id,
        isEliminated: false,
        isRound1: gameState.round === 'round1',
        isRound2: gameState.round === 'round2',
        isRound3: gameState.round === 'round3',
        isPointingPhase: false,
        isMyTurnToPoint: false,
        currentPhase,
        statusMessage: 'Test Status',
        activePlayerName: 'Player 1',
        pointerName: 'Player 1',
        deciderName: 'Player 1',
        ...logicMock.overrides
      }
    }
  }
})

function makeContestant(overrides: Partial<Contestant> = {}): Contestant {
  return {
    id: 'c1',
    name: 'Player 1',
    score: 0,
    lives: 3,
    round1_misses: 0,
    round1_questions: 0,
    online: true,
    eliminated: false,
    ready: false,
    session_id: 's1',
    age: '25',
    ...overrides
  }
}

describe('ContestantScreen', () => {
  const mockGameState: GameStateSnapshot = {
    round: 'lobby',
    contestants: [makeContestant()],
    active_player_id: undefined,
    current_question: undefined,
    timer_start: undefined,
    has_presenter: false,
    presenter_online: false,
    decision_pending: false
  }

  const defaultProps = {
    gameState: mockGameState,
    contestantName: 'Player 1',
    sessionId: 'c1',
    onToggleReady: vi.fn(),
    onSubmitAnswer: vi.fn(),
    pointToPlayer: vi.fn(),
    buzzIn: vi.fn(),
    makeDecision: vi.fn()
  }

  const question: Question = {
    text: 'Q1',
    correct_answer: 'A',
    options: []
  }

  beforeEach(() => {
    vi.clearAllMocks()
    logicMock.overrides = {}
  })

  it('renders lobby view', () => {
    render(ContestantScreen, { ...defaultProps })
    expect(screen.getByText('Are you ready to play?')).toBeInTheDocument()
  })

  it('renders answering view when active in round 1', () => {
    const round1State: GameStateSnapshot = {
      ...mockGameState,
      round: 'round1',
      active_player_id: 'c1',
      current_question: question
    }
    render(ContestantScreen, { ...defaultProps, gameState: round1State })
    expect(screen.getByText("IT'S YOUR TURN!")).toBeInTheDocument()
  })

  it('hides the stats footer in the lobby and shows it once a round starts', () => {
    const { unmount } = render(ContestantScreen, { ...defaultProps })
    expect(screen.queryByText('Score')).not.toBeInTheDocument()
    unmount()

    render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        contestants: [
          makeContestant({ score: 120, lives: 2, round1_misses: 1 })
        ]
      }
    })

    expect(screen.getByText('Score')).toBeInTheDocument()
    expect(screen.getByText('120')).toBeInTheDocument()
    expect(screen.getByText('❤️❤️')).toBeInTheDocument()
    // Round 1 also exposes the strike counter
    expect(screen.getByText('1/2')).toBeInTheDocument()
  })

  it('falls back to zero score and lives when the contestant is not in the snapshot', () => {
    render(ContestantScreen, {
      ...defaultProps,
      sessionId: 'not-in-game',
      gameState: { ...mockGameState, round: 'round2' }
    })

    expect(screen.getByText('0')).toBeInTheDocument()
    const lives = screen.getByText('Lives').nextElementSibling
    expect(lives).toHaveTextContent('')
  })

  it('renders the presenter status from the game snapshot', () => {
    render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        has_presenter: true,
        presenter_online: true
      }
    })

    expect(screen.getByText(/Presenter Online/)).toBeInTheDocument()
    expect(screen.getByText('Test Status')).toBeInTheDocument()
  })

  it('renders the eliminated view with the final score', () => {
    logicMock.overrides = {
      currentPhase: 'ELIMINATED',
      isEliminated: true,
      isRound3: true
    }

    render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round3',
        contestants: [makeContestant({ score: 250 })]
      }
    })

    expect(screen.getByText('ELIMINATED')).toBeInTheDocument()
    expect(screen.getByText('Current Round: The Buzzer!')).toBeInTheDocument()
    expect(screen.getByText('Final Score: 250')).toBeInTheDocument()
  })

  it('shows a zero final score when the eliminated contestant is gone from the snapshot', () => {
    logicMock.overrides = {
      currentPhase: 'ELIMINATED',
      isEliminated: true,
      isRound3: false
    }

    render(ContestantScreen, {
      ...defaultProps,
      sessionId: 'not-in-game',
      gameState: { ...mockGameState, round: 'round2' }
    })

    expect(screen.getByText('Final Score: 0')).toBeInTheDocument()
    expect(
      screen.queryByText('Current Round: The Buzzer!')
    ).not.toBeInTheDocument()
  })

  it('lets the pointing player pick a target in round 2', async () => {
    logicMock.overrides = { currentPhase: 'POINTING', isMyTurnToPoint: true }
    const pointToPlayer = vi.fn()

    render(ContestantScreen, {
      ...defaultProps,
      pointToPlayer,
      gameState: {
        ...mockGameState,
        round: 'round2',
        decision_pending: true,
        contestants: [
          makeContestant(),
          makeContestant({ id: 'c2', name: 'Player 2', session_id: 's2' })
        ]
      }
    })

    expect(
      screen.getByText("It's your turn to choose the next player!")
    ).toBeInTheDocument()

    await fireEvent.click(screen.getByText('Player 2'))
    expect(pointToPlayer).toHaveBeenCalledWith('c2')
  })

  it('shows the pointing spectator view when it is somebody else’s turn', () => {
    logicMock.overrides = { currentPhase: 'POINTING', isMyTurnToPoint: false }

    render(ContestantScreen, {
      ...defaultProps,
      gameState: { ...mockGameState, round: 'round2', decision_pending: true }
    })

    expect(screen.getByText('Pointing Phase')).toBeInTheDocument()
    expect(
      screen.getByText('Waiting for Player 1 to select a player...')
    ).toBeInTheDocument()
  })

  it('wires the buzzer to the buzzIn action', async () => {
    logicMock.overrides = { currentPhase: 'BUZZER' }
    const buzzIn = vi.fn()

    render(ContestantScreen, {
      ...defaultProps,
      buzzIn,
      gameState: { ...mockGameState, round: 'round3' }
    })

    await fireEvent.click(screen.getByText('BUZZ!'))
    expect(buzzIn).toHaveBeenCalledTimes(1)
  })

  it('wires the round 3 decision to the makeDecision action', async () => {
    logicMock.overrides = { currentPhase: 'DECISION' }
    const makeDecision = vi.fn()

    render(ContestantScreen, {
      ...defaultProps,
      makeDecision,
      gameState: {
        ...mockGameState,
        round: 'round3',
        decision_pending: true,
        contestants: [
          makeContestant(),
          makeContestant({ id: 'c2', name: 'Player 2', session_id: 's2' })
        ]
      }
    })

    await fireEvent.click(screen.getByText('Double Down (Self)'))
    expect(makeDecision).toHaveBeenCalledWith('self')

    await fireEvent.click(screen.getByText('Point to Player'))
    await fireEvent.click(screen.getByText('Player 2'))
    expect(makeDecision).toHaveBeenLastCalledWith('point', 'c2')
  })

  it('shows who is deciding while spectating a round 3 decision', () => {
    logicMock.overrides = { currentPhase: 'SPECTATING_DECISION' }

    render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round3',
        decision_pending: true
      }
    })

    expect(screen.getByText('Decision Time')).toBeInTheDocument()
    expect(
      screen.getByText('Player 1 is making a decision...')
    ).toBeInTheDocument()
  })

  it('shows the host speaking view while waiting for the presenter', () => {
    logicMock.overrides = { currentPhase: 'WAITING_FOR_PRESENTER' }

    render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        waiting_for_presenter: true
      }
    })

    expect(screen.getByText('Host Speaking')).toBeInTheDocument()
    expect(screen.getByText('Listen to the presenter...')).toBeInTheDocument()
  })

  it('renders the waiting view for a non active player in round 1', () => {
    render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        active_player_id: 'c2',
        current_question: question
      }
    })

    expect(screen.getByText('Waiting for other players...')).toBeInTheDocument()
    expect(screen.getByText(/Active Player: Player 1/)).toBeInTheDocument()
    expect(
      screen.queryByPlaceholderText('Type your answer...')
    ).not.toBeInTheDocument()
  })

  it('numbers the question from the answered count and omits it when unknown', () => {
    const { unmount } = render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        active_player_id: 'c1',
        current_question: question,
        contestants: [makeContestant({ round1_questions: 2 })]
      }
    })

    expect(screen.getByText('Question 3')).toBeInTheDocument()
    unmount()

    const withoutCount = makeContestant()
    delete (withoutCount as Partial<Contestant>).round1_questions
    render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        active_player_id: 'c1',
        current_question: question,
        contestants: [withoutCount]
      }
    })

    expect(screen.getByText('Q1')).toBeInTheDocument()
    expect(screen.queryByText(/^Question \d+$/)).not.toBeInTheDocument()
  })

  it('submits a timeout answer when the active player runs out of time', async () => {
    const onSubmitAnswer = vi.fn()

    render(ContestantScreen, {
      ...defaultProps,
      onSubmitAnswer,
      gameState: {
        ...mockGameState,
        round: 'round1',
        active_player_id: 'c1',
        current_question: question,
        // Started 61s ago, so the 60s timer is already expired
        timer_start: Date.now() / 1000 - 61
      }
    })

    await waitFor(() => {
      expect(onSubmitAnswer).toHaveBeenCalledWith('!!!TIMEOUT!!!')
    })
  })

  it('celebrates the session owner when they win', () => {
    const { container } = render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'finished',
        winner_id: 'c1',
        contestants: [makeContestant({ score: 300 })]
      }
    })

    expect(screen.getByText('Game Over')).toBeInTheDocument()
    expect(screen.getByText('Congratulations! 🎉')).toBeInTheDocument()
    expect(
      screen.getByText('You are the winner of 1 z 10!')
    ).toBeInTheDocument()
    expect(container.querySelector('.winner-message')).toHaveClass('victory')
    expect(container.querySelector('.final-score')).toHaveTextContent(
      'Your Score: 300 points'
    )
    expect(
      screen.getByText(/Your Final Score:\s*300\s*pts/)
    ).toBeInTheDocument()
    expect(screen.queryByText('Winner Announcement')).not.toBeInTheDocument()
  })

  it('shows a zero score for a winner who scored nothing', () => {
    const { container } = render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'finished',
        winner_id: 'c1',
        contestants: [makeContestant({ score: 0 })]
      }
    })

    expect(container.querySelector('.final-score')).toHaveTextContent(
      'Your Score: 0 points'
    )
    expect(screen.getByText(/Your Final Score:\s*0\s*pts/)).toBeInTheDocument()
  })

  it('announces another contestant as the winner', () => {
    const { container } = render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'finished',
        winner_id: 'c2',
        contestants: [
          makeContestant({ score: 40 }),
          makeContestant({
            id: 'c2',
            name: 'Player 2',
            session_id: 's2',
            score: 310
          })
        ]
      }
    })

    expect(screen.getByText('Winner Announcement')).toBeInTheDocument()
    expect(container.querySelector('.winner-name')).toHaveTextContent(
      'Winner: Player 2'
    )
    expect(container.querySelector('.winner-points')).toHaveTextContent(
      'Score: 310 points'
    )
    expect(container.querySelector('.winner-message')).not.toHaveClass(
      'victory'
    )
    expect(screen.getByText(/Your Final Score:\s*40\s*pts/)).toBeInTheDocument()
    expect(screen.queryByText('Congratulations! 🎉')).not.toBeInTheDocument()
  })

  it('falls back to an unknown winner when the winner id is not in the snapshot', () => {
    const { container } = render(ContestantScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'finished',
        winner_id: 'ghost'
      }
    })

    expect(container.querySelector('.winner-name')).toHaveTextContent(
      'Winner: Unknown'
    )
    expect(container.querySelector('.winner-points')).toHaveTextContent(
      'Score: 0 points'
    )
  })

  it('falls back to the game ended screen for an unhandled phase', () => {
    // Defensive branch: guards against a phase the template does not know about.
    logicMock.overrides = { currentPhase: 'SOMETHING_NEW' }

    render(ContestantScreen, {
      ...defaultProps,
      gameState: { ...mockGameState, round: 'round1' }
    })

    expect(screen.getByText('Game Ended.')).toBeInTheDocument()
  })
})
