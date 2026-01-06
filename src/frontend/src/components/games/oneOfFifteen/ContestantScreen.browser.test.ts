// @vitest-environment jsdom
import { render, screen } from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import ContestantScreen from './ContestantScreen.svelte'
import type {
  GameStateSnapshot,
  Question
} from '../../../hooks/useOneOfFifteenState.svelte'
import '@testing-library/jest-dom' // Fixes toBeInTheDocument

// Mock dependencies
vi.mock('../../../hooks/useContestantLogic.svelte', () => {
  const PHASE = {
    LOBBY: 'LOBBY',
    ELIMINATED: 'ELIMINATED',
    POINTING: 'POINTING',
    BUZZER: 'BUZZER',
    DECISION: 'DECISION',
    ANSWERING: 'ANSWERING',
    WAITING: 'WAITING',
    SPECTATING_DECISION: 'SPECTATING_DECISION',
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

      let currentPhase = PHASE.LOBBY
      if (gameState.round === 'round1') {
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
        deciderName: 'Player 1'
      }
    }
  }
})

describe('ContestantScreen', () => {
  const mockGameState: GameStateSnapshot = {
    round: 'lobby',
    contestants: [
      {
        id: 'c1',
        name: 'Player 1',
        score: 0,
        lives: 3,
        round1_misses: 0,
        online: true,
        eliminated: false,
        ready: false,
        session_id: 's1',
        age: '25',
        round1_questions: 0
      }
    ],
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

  beforeEach(() => {
    vi.clearAllMocks()
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
      current_question: {
        text: 'Q1',
        options: [],
        correct_answer: 'A',
        difficulty: 'easy'
      } as unknown as Question // Cast to avoid strict type check on missing props if any
    }
    render(ContestantScreen, { ...defaultProps, gameState: round1State })
    expect(screen.getByText("IT'S YOUR TURN!")).toBeInTheDocument()
  })
})
