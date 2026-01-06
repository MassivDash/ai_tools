// @vitest-environment jsdom
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import PresenterScreen from './PresenterScreen.svelte'
import type { GameStateSnapshot } from '../../../hooks/useOneOfFifteenState.svelte'

// Mock dependencies

// Mock usePresenterSpeech
vi.mock('../../../hooks/usePresenterSpeech.svelte', () => ({
  usePresenterSpeech: () => ({
    speak: vi.fn(),
    speakAndWait: vi.fn().mockResolvedValue(undefined),
    cancel: vi.fn(),
    isSpeaking: false,
    robotTalking: false
  })
}))

// Mock presenterService
vi.mock('../../../api/games/oneOfFifteen/presenterService', () => ({
  generateIntroSpeech: vi.fn().mockResolvedValue('Welcome humans!')
}))

describe('PresenterScreen', () => {
  const mockGameState: GameStateSnapshot = {
    round: 'lobby',
    contestants: [
      {
        id: 'c1',
        name: 'Alice',
        score: 0,
        lives: 3,
        round1_misses: 0,
        online: true,
        eliminated: false,
        ready: true,
        session_id: 's1'
      }
    ],
    active_player_id: null,
    current_question: null,
    timer_start: null,
    has_presenter: true,
    presenter_online: true,
    decision_pending: false,
    buzzer_queue: []
  }

  const defaultProps = {
    gameState: mockGameState,
    onStartGame: vi.fn(),
    onResetGame: vi.fn()
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders lobby state correctly', () => {
    render(PresenterScreen, { ...defaultProps })

    expect(screen.getByText('Game Controls')).toBeInTheDocument()
    expect(screen.getByText('Start Game')).toBeInTheDocument()
    expect(screen.getByText('Contestants (1)')).toBeInTheDocument()
    expect(screen.getByText('Alice')).toBeInTheDocument()
    expect(screen.getByText('Waiting for Game to Start...')).toBeInTheDocument()
  })

  it('handles start game interaction', async () => {
    const onStartGame = vi.fn()
    render(PresenterScreen, { ...defaultProps, onStartGame })

    const startBtn = screen.getByText('Start Game')
    await fireEvent.click(startBtn)

    // It triggers intro generation (mocked), then calls onStartGame
    // The sequence is: click -> generateIntro (async) -> speakAndWait (async) -> onStartGame

    // We wait for the final effect
    await waitFor(() => {
      expect(onStartGame).toHaveBeenCalled()
    })
  })

  it('renders round 1 state correctly', () => {
    const round1State: GameStateSnapshot = {
      ...mockGameState,
      round: 'round1',
      current_question: {
        text: 'What is 1+1?',
        options: [],
        correct_answer: '2',
        category: 'Math',
        difficulty: 'easy'
      }
    }

    render(PresenterScreen, { ...defaultProps, gameState: round1State })

    expect(screen.getByText('ROUND1')).toBeInTheDocument()
    expect(screen.getByText('Current Question:')).toBeInTheDocument()
    expect(screen.getByText('What is 1+1?')).toBeInTheDocument()
    expect(screen.getByText('2')).toBeInTheDocument()
    expect(
      screen.queryByText('Waiting for Game to Start...')
    ).not.toBeInTheDocument()
  })

  it('renders reset button when game is active', () => {
    const activeState: GameStateSnapshot = {
      ...mockGameState,
      round: 'round1'
    }

    render(PresenterScreen, { ...defaultProps, gameState: activeState })

    expect(screen.getByText('Reset Game')).toBeInTheDocument()
    expect(screen.queryByText('Start Game')).not.toBeInTheDocument()
  })
})
