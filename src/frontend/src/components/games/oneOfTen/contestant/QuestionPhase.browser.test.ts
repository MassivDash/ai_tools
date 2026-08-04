// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, act } from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import QuestionPhase from './QuestionPhase.svelte'
import type { Question } from '../../../../hooks/useOneOfTenState.svelte'

const question: Question = {
  text: 'What is the capital of Poland?',
  correct_answer: 'Warsaw'
}

describe('QuestionPhase', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.runOnlyPendingTimers()
    vi.useRealTimers()
  })

  it('shows the question, numbered, with a timer and answer box for the active player', async () => {
    const onSubmitAnswer = vi.fn()
    render(QuestionPhase, {
      props: {
        isActivePlayer: true,
        timerStart: Date.now() / 1000,
        duration: 60,
        currentQuestion: question,
        questionNumber: 3,
        onTimeout: vi.fn(),
        onSubmitAnswer
      }
    })

    expect(screen.getByText("IT'S YOUR TURN!")).toBeInTheDocument()
    expect(screen.getByText('Question 3')).toBeInTheDocument()
    expect(screen.getByText(question.text)).toBeInTheDocument()
    expect(screen.getByText('60')).toBeInTheDocument()
    expect(screen.queryByText('Generating question...')).not.toBeInTheDocument()

    await fireEvent.input(screen.getByPlaceholderText('Type your answer...'), {
      target: { value: 'Warsaw' }
    })
    await fireEvent.click(screen.getByText('Submit'))
    expect(onSubmitAnswer).toHaveBeenCalledWith('Warsaw')
  })

  it('shows the generating placeholder while the active player has no question yet', () => {
    render(QuestionPhase, {
      props: {
        isActivePlayer: true,
        timerStart: undefined,
        duration: 60,
        currentQuestion: null,
        onTimeout: vi.fn(),
        onSubmitAnswer: vi.fn()
      }
    })

    expect(screen.getByText("IT'S YOUR TURN!")).toBeInTheDocument()
    expect(screen.getByText('Generating question...')).toBeInTheDocument()
    expect(
      screen.queryByPlaceholderText('Type your answer...')
    ).not.toBeInTheDocument()
  })

  it('forwards the timer expiry to onTimeout', async () => {
    const onTimeout = vi.fn()
    render(QuestionPhase, {
      props: {
        isActivePlayer: true,
        timerStart: Date.now() / 1000,
        duration: 5,
        currentQuestion: question,
        onTimeout,
        onSubmitAnswer: vi.fn()
      }
    })

    await act(() => {
      vi.advanceTimersByTime(5000)
    })

    expect(screen.getByText('0')).toBeInTheDocument()
    expect(onTimeout).toHaveBeenCalled()
  })

  it('shows the spectator view with the active player name and the question', () => {
    render(QuestionPhase, {
      props: {
        isActivePlayer: false,
        timerStart: Date.now() / 1000,
        duration: 60,
        currentQuestion: question,
        activePlayerName: 'Alice',
        onTimeout: vi.fn(),
        onSubmitAnswer: vi.fn()
      }
    })

    expect(screen.getByText('Waiting for other players...')).toBeInTheDocument()
    expect(screen.getByText(/Active Player: Alice/)).toBeInTheDocument()
    expect(screen.getByText(question.text)).toBeInTheDocument()
    // Spectators get neither timer nor answer box
    expect(screen.queryByText('seconds')).not.toBeInTheDocument()
    expect(
      screen.queryByPlaceholderText('Type your answer...')
    ).not.toBeInTheDocument()
  })

  it('falls back to Unknown and hides the question card when spectating with no data', () => {
    render(QuestionPhase, {
      props: {
        isActivePlayer: false,
        timerStart: null,
        duration: 60,
        currentQuestion: undefined,
        activePlayerName: undefined,
        onTimeout: vi.fn(),
        onSubmitAnswer: vi.fn()
      }
    })

    expect(screen.getByText(/Active Player: Unknown/)).toBeInTheDocument()
    expect(screen.queryByText(question.text)).not.toBeInTheDocument()
  })
})
