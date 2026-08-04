// @vitest-environment jsdom
import {
  render,
  screen,
  fireEvent,
  waitFor,
  act
} from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import PresenterScreen from './PresenterScreen.svelte'
import type {
  Contestant,
  GameStateSnapshot
} from '../../../hooks/useOneOfTenState.svelte'

// Mock dependencies

const speechMocks = vi.hoisted(() => ({
  speak: vi.fn(),
  speakAndWait: vi.fn()
}))

const serviceMocks = vi.hoisted(() => ({
  generateIntroSpeech: vi.fn(),
  generateHostJoke: vi.fn(),
  generateAnswerComment: vi.fn(),
  generateWinnerSpeech: vi.fn()
}))

// Mock usePresenterSpeech
vi.mock('../../../hooks/usePresenterSpeech.svelte', () => ({
  usePresenterSpeech: () => ({
    speak: speechMocks.speak,
    speakAndWait: speechMocks.speakAndWait,
    cancel: vi.fn(),
    isSpeaking: false,
    robotTalking: false
  })
}))

// Mock presenterService
vi.mock('../../../api/games/oneOfTen/presenterService', () => serviceMocks)

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
    ready: true,
    session_id: 's1',
    ...overrides
  }
}

describe('PresenterScreen', () => {
  const mockGameState: GameStateSnapshot = {
    round: 'lobby',
    contestants: [makeContestant()],
    active_player_id: undefined,
    current_question: undefined,
    timer_start: undefined,
    has_presenter: true,
    presenter_online: true,
    decision_pending: false
  }

  const defaultProps = {
    gameState: mockGameState,
    onStartGame: vi.fn(),
    onResetGame: vi.fn(),
    onPresenterFinishedSpeaking: vi.fn()
  }

  beforeEach(() => {
    vi.clearAllMocks()
    speechMocks.speakAndWait.mockResolvedValue(undefined)
    serviceMocks.generateIntroSpeech.mockResolvedValue('Welcome humans!')
    serviceMocks.generateHostJoke.mockResolvedValue('Ha ha ha!')
    serviceMocks.generateAnswerComment.mockResolvedValue('Good job!')
    serviceMocks.generateWinnerSpeech.mockResolvedValue('Congratulations!')
  })

  afterEach(() => {
    vi.restoreAllMocks()
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
    // Wait for the async flow to finish
    await waitFor(() => {
      expect(onStartGame).toHaveBeenCalled()
    })
  })

  it('speaks the intro then the joke before starting the game', async () => {
    const onStartGame = vi.fn()
    render(PresenterScreen, { ...defaultProps, onStartGame })

    await fireEvent.click(screen.getByText('Start Game'))

    await waitFor(() => {
      expect(onStartGame).toHaveBeenCalled()
    })

    expect(serviceMocks.generateIntroSpeech).toHaveBeenCalledWith(
      mockGameState.contestants
    )
    expect(serviceMocks.generateHostJoke).toHaveBeenCalled()
    expect(speechMocks.speakAndWait.mock.calls.map((c) => c[0])).toEqual([
      'Welcome humans!',
      'Ha ha ha!'
    ])
  })

  it('refuses to start the game with no contestants', async () => {
    const onStartGame = vi.fn()
    render(PresenterScreen, {
      ...defaultProps,
      onStartGame,
      gameState: { ...mockGameState, contestants: [] }
    })

    expect(screen.getByText('Contestants (0)')).toBeInTheDocument()
    const startBtn = screen.getByRole('button', { name: /Start Game/ })
    expect(startBtn).toBeDisabled()

    // fireEvent bypasses the disabled attribute, which exercises the guard
    // clause inside handleStartGame itself.
    await fireEvent.click(startBtn)

    expect(serviceMocks.generateIntroSpeech).not.toHaveBeenCalled()
    expect(onStartGame).not.toHaveBeenCalled()
  })

  it('renders round 1 state correctly', () => {
    const round1State: GameStateSnapshot = {
      ...mockGameState,
      round: 'round1',
      current_question: {
        text: 'What is 1+1?',
        options: [],
        correct_answer: '2'
      }
    }

    render(PresenterScreen, { ...defaultProps, gameState: round1State })

    expect(screen.getByText('ROUND1')).toBeInTheDocument()
    expect(screen.getByText('Current Question:')).toBeInTheDocument()
    expect(screen.getByText('What is 1+1?')).toBeInTheDocument()
    expect(screen.queryByText('2')).not.toBeInTheDocument()
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

  it('announces the start of round 1 and speaks the question once', async () => {
    render(PresenterScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: { text: 'Q1', correct_answer: 'A' }
      }
    })

    await waitFor(() => {
      expect(speechMocks.speakAndWait).toHaveBeenCalledWith('Start of Round 1')
    })
    await waitFor(() => {
      expect(speechMocks.speak).toHaveBeenCalledWith('Q1')
    })
    expect(speechMocks.speak).toHaveBeenCalledTimes(1)
  })

  it('announces round 2', async () => {
    render(PresenterScreen, {
      ...defaultProps,
      gameState: { ...mockGameState, round: 'round2' }
    })

    await waitFor(() => {
      expect(speechMocks.speakAndWait).toHaveBeenCalledWith(
        "Let's start Round 2"
      )
    })
    expect(screen.getByText('ROUND2')).toBeInTheDocument()
  })

  it('announces round 3', async () => {
    render(PresenterScreen, {
      ...defaultProps,
      gameState: { ...mockGameState, round: 'round3' }
    })

    await waitFor(() => {
      expect(speechMocks.speakAndWait).toHaveBeenCalledWith('Final Round 3')
    })
    expect(screen.getByText('ROUND3')).toBeInTheDocument()
  })

  it('handles game finished state and announces winner', async () => {
    const finishedState: GameStateSnapshot = {
      ...mockGameState,
      round: 'finished',
      winner_id: 'c1',
      contestants: [makeContestant({ score: 90 })]
    }

    const { container } = render(PresenterScreen, {
      ...defaultProps,
      gameState: finishedState
    })

    await waitFor(() => {
      expect(serviceMocks.generateWinnerSpeech).toHaveBeenCalledWith(
        'Alice',
        90
      )
    })
    await waitFor(() => {
      expect(speechMocks.speakAndWait).toHaveBeenCalledWith('Congratulations!')
    })
    expect(screen.getByText('We Have a Winner!')).toBeInTheDocument()
    expect(container.querySelector('.winner-name')).toHaveTextContent('Alice')
    expect(screen.getByText('90 Points')).toBeInTheDocument()
  })

  it('falls back to a generic closing speech when no winner is known', async () => {
    render(PresenterScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'finished',
        winner_id: 'nobody'
      }
    })

    await waitFor(() => {
      expect(speechMocks.speakAndWait).toHaveBeenCalledWith(
        'The game is finished! Thank you all for playing.'
      )
    })
    expect(serviceMocks.generateWinnerSpeech).not.toHaveBeenCalled()
    expect(screen.getByText('No Winner')).toBeInTheDocument()
    expect(screen.getByText('0 Points')).toBeInTheDocument()
  })

  it('comments on a correct answer and reports back when it is done speaking', async () => {
    const onPresenterFinishedSpeaking = vi.fn()
    const { rerender } = render(PresenterScreen, {
      ...defaultProps,
      onPresenterFinishedSpeaking,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: { text: 'Capital of Poland?', correct_answer: 'W' }
      }
    })

    await waitFor(() => {
      expect(speechMocks.speak).toHaveBeenCalledWith('Capital of Poland?')
    })

    // Round logic clears the question and reports the verdict
    await rerender({
      ...defaultProps,
      onPresenterFinishedSpeaking,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: undefined,
        last_answer_correct: true,
        last_correct_answer: 'Warsaw'
      }
    })

    await waitFor(() => {
      expect(serviceMocks.generateAnswerComment).toHaveBeenCalledWith(
        'Capital of Poland?',
        true,
        'Warsaw'
      )
    })
    await waitFor(() => {
      expect(onPresenterFinishedSpeaking).toHaveBeenCalledTimes(1)
    })
    expect(speechMocks.speakAndWait).toHaveBeenCalledWith('Good job!')
  })

  it('looks sad while commenting on a wrong answer and recovers afterwards', async () => {
    let resolveComment: (_value: string) => void = () => {}
    serviceMocks.generateAnswerComment.mockReturnValue(
      new Promise<string>((resolve) => {
        resolveComment = resolve
      })
    )

    const onPresenterFinishedSpeaking = vi.fn()
    const { container, rerender } = render(PresenterScreen, {
      ...defaultProps,
      onPresenterFinishedSpeaking,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: { text: 'Q1', correct_answer: 'A' }
      }
    })

    await waitFor(() => {
      expect(speechMocks.speak).toHaveBeenCalledWith('Q1')
    })

    await rerender({
      ...defaultProps,
      onPresenterFinishedSpeaking,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: undefined,
        last_answer_correct: false,
        last_correct_answer: 'A'
      }
    })

    await waitFor(() => {
      expect(container.querySelector('.robot-head')).toHaveClass('sad')
    })
    expect(serviceMocks.generateAnswerComment).toHaveBeenCalledWith(
      'Q1',
      false,
      'A'
    )

    resolveComment('Wrong, sorry!')

    await waitFor(() => {
      expect(onPresenterFinishedSpeaking).toHaveBeenCalled()
    })
    await waitFor(() => {
      expect(container.querySelector('.robot-head')).toHaveClass('normal')
    })
  })

  it('still reports back when generating the answer comment fails', async () => {
    const consoleError = vi
      .spyOn(console, 'error')
      .mockImplementation(() => undefined)
    serviceMocks.generateAnswerComment.mockRejectedValue(new Error('LLM down'))

    const onPresenterFinishedSpeaking = vi.fn()
    const { rerender } = render(PresenterScreen, {
      ...defaultProps,
      onPresenterFinishedSpeaking,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: { text: 'Q1', correct_answer: 'A' }
      }
    })

    await waitFor(() => {
      expect(speechMocks.speak).toHaveBeenCalledWith('Q1')
    })

    await rerender({
      ...defaultProps,
      onPresenterFinishedSpeaking,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: undefined,
        last_answer_correct: true
      }
    })

    await waitFor(() => {
      expect(onPresenterFinishedSpeaking).toHaveBeenCalledTimes(1)
    })
    expect(consoleError).toHaveBeenCalledWith(
      'Answer feedback failed',
      expect.any(Error)
    )
  })

  it('stays quiet when there is no answer verdict to comment on', async () => {
    const { rerender } = render(PresenterScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: undefined,
        // No question has been spoken yet, so there is nothing to comment on
        last_answer_correct: true
      }
    })

    await waitFor(() => {
      expect(speechMocks.speakAndWait).toHaveBeenCalledWith('Start of Round 1')
    })
    expect(serviceMocks.generateAnswerComment).not.toHaveBeenCalled()

    // A question is on screen: still nothing to comment on
    await rerender({
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: { text: 'Q1', correct_answer: 'A' },
        last_answer_correct: true
      }
    })
    await waitFor(() => {
      expect(speechMocks.speak).toHaveBeenCalledWith('Q1')
    })
    expect(serviceMocks.generateAnswerComment).not.toHaveBeenCalled()

    // Question cleared but no verdict yet
    await rerender({
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: undefined,
        last_answer_correct: undefined
      }
    })
    expect(serviceMocks.generateAnswerComment).not.toHaveBeenCalled()
  })

  it('comments on each answered question only once', async () => {
    const onPresenterFinishedSpeaking = vi.fn()
    const answeredState: GameStateSnapshot = {
      ...mockGameState,
      round: 'round1',
      current_question: undefined,
      last_answer_correct: true,
      last_correct_answer: 'A'
    }
    const { rerender } = render(PresenterScreen, {
      ...defaultProps,
      onPresenterFinishedSpeaking,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: { text: 'Q1', correct_answer: 'A' }
      }
    })

    await waitFor(() => {
      expect(speechMocks.speak).toHaveBeenCalledWith('Q1')
    })

    await rerender({
      ...defaultProps,
      onPresenterFinishedSpeaking,
      gameState: answeredState
    })
    await waitFor(() => {
      expect(serviceMocks.generateAnswerComment).toHaveBeenCalledTimes(1)
    })

    // Same snapshot re-delivered by the websocket must not re-trigger feedback
    await rerender({
      ...defaultProps,
      onPresenterFinishedSpeaking,
      gameState: { ...answeredState }
    })
    await waitFor(() => {
      expect(onPresenterFinishedSpeaking).toHaveBeenCalledTimes(1)
    })
    expect(serviceMocks.generateAnswerComment).toHaveBeenCalledTimes(1)
  })

  it('waits for the answer feedback to finish before reading the next question', async () => {
    let resolveComment: (_value: string) => void = () => {}
    serviceMocks.generateAnswerComment.mockReturnValue(
      new Promise<string>((resolve) => {
        resolveComment = resolve
      })
    )

    const { rerender } = render(PresenterScreen, {
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: { text: 'Q1', correct_answer: 'A' }
      }
    })

    await waitFor(() => {
      expect(speechMocks.speak).toHaveBeenCalledWith('Q1')
    })

    // Answer submitted: feedback starts and stays in flight
    await rerender({
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: undefined,
        last_answer_correct: true
      }
    })
    await waitFor(() => {
      expect(serviceMocks.generateAnswerComment).toHaveBeenCalled()
    })

    // Next question arrives while the robot is still commenting
    await rerender({
      ...defaultProps,
      gameState: {
        ...mockGameState,
        round: 'round1',
        current_question: { text: 'Q2', correct_answer: 'B' }
      }
    })
    await new Promise((resolve) => setTimeout(resolve, 250))
    expect(speechMocks.speak).toHaveBeenCalledTimes(1)

    resolveComment('Good job!')

    await waitFor(() => {
      expect(speechMocks.speak).toHaveBeenCalledWith('Q2')
    })
  })

  describe('round 1 timer', () => {
    beforeEach(() => {
      vi.useFakeTimers()
    })

    afterEach(() => {
      vi.runOnlyPendingTimers()
      vi.useRealTimers()
    })

    it('counts down from 60 and turns urgent below 10 seconds', async () => {
      const { container } = render(PresenterScreen, {
        ...defaultProps,
        gameState: {
          ...mockGameState,
          round: 'round1',
          timer_start: Date.now() / 1000
        }
      })

      expect(screen.getByText('⏰ 60s')).toBeInTheDocument()
      expect(container.querySelector('.timer-display')).not.toHaveClass(
        'urgent'
      )

      await act(() => {
        vi.advanceTimersByTime(5000)
      })
      expect(screen.getByText('⏰ 55s')).toBeInTheDocument()

      await act(() => {
        vi.advanceTimersByTime(46000)
      })
      expect(screen.getByText('⏰ 9s')).toBeInTheDocument()
      expect(container.querySelector('.timer-display')).toHaveClass('urgent')

      // Never goes below zero
      await act(() => {
        vi.advanceTimersByTime(30000)
      })
      expect(screen.getByText('⏰ 0s')).toBeInTheDocument()
    })

    it('looks happy while reading the question and relaxes 3 seconds later', async () => {
      const { container } = render(PresenterScreen, {
        ...defaultProps,
        gameState: {
          ...mockGameState,
          round: 'round1',
          current_question: { text: 'Q1', correct_answer: 'A' }
        }
      })

      await act(() => vi.advanceTimersByTimeAsync(0))
      expect(speechMocks.speak).toHaveBeenCalledWith('Q1')
      expect(container.querySelector('.robot-head')).toHaveClass('happy')

      await act(() => vi.advanceTimersByTimeAsync(3000))
      expect(container.querySelector('.robot-head')).toHaveClass('normal')
    })

    it('resets the countdown to 60 when the timer stops', async () => {
      const { rerender } = render(PresenterScreen, {
        ...defaultProps,
        gameState: {
          ...mockGameState,
          round: 'round1',
          timer_start: Date.now() / 1000
        }
      })

      await act(() => {
        vi.advanceTimersByTime(10000)
      })
      expect(screen.getByText('⏰ 50s')).toBeInTheDocument()

      await act(() =>
        rerender({
          ...defaultProps,
          gameState: {
            ...mockGameState,
            round: 'round1',
            timer_start: undefined
          }
        })
      )

      expect(screen.getByText('⏰ 60s')).toBeInTheDocument()
    })
  })
})
