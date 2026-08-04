/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, fireEvent, screen } from '@testing-library/svelte'
import { expect, test, vi, describe, beforeEach, afterEach } from 'vitest'
import VoiceInput from './VoiceInput.svelte'
import * as useSpeechRecognition from '@hooks/useSpeechRecognition.svelte'

// Mock the hook
vi.mock('@hooks/useSpeechRecognition.svelte', () => ({
  useSpeechRecognition: vi.fn()
}))

const ALWAYS_ON_TITLE = 'Always On: Auto-restart after sending'

describe('VoiceInput Component', () => {
  let mockSpeech: any

  // The component wires its own callbacks into the hook; grabbing the options
  // object it passed lets the tests drive the recognition events.
  const speechOptions = () => {
    const calls = (useSpeechRecognition.useSpeechRecognition as any).mock.calls
    return calls[calls.length - 1][0]
  }

  beforeEach(() => {
    ;(useSpeechRecognition.useSpeechRecognition as any).mockClear()
    mockSpeech = {
      isSupported: true,
      isListening: false,
      error: null,
      start: vi.fn(),
      stop: vi.fn(),
      toggle: vi.fn()
    }
    ;(useSpeechRecognition.useSpeechRecognition as any).mockReturnValue(
      mockSpeech
    )
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  test('renders start button when supported', () => {
    const { getByTitle } = render(VoiceInput, {
      props: {
        onTranscript: vi.fn(),
        onCommand: vi.fn()
      }
    })

    expect(getByTitle('Start Voice Input')).toBeTruthy()
  })

  test('renders stop button when listening', () => {
    mockSpeech.isListening = true
    const { getByTitle } = render(VoiceInput, {
      props: {
        onTranscript: vi.fn(),
        onCommand: vi.fn()
      }
    })

    expect(getByTitle('Stop Listening')).toBeTruthy()
  })

  test('toggles speech on click', async () => {
    const { getByTitle } = render(VoiceInput, {
      props: {
        onTranscript: vi.fn(),
        onCommand: vi.fn()
      }
    })

    const button = getByTitle('Start Voice Input')
    await fireEvent.click(button)

    expect(mockSpeech.toggle).toHaveBeenCalled()
  })

  test('toggles always on mode', async () => {
    const { getByTitle } = render(VoiceInput, {
      props: {
        onTranscript: vi.fn(),
        onCommand: vi.fn()
      }
    })

    const button = getByTitle(ALWAYS_ON_TITLE)
    expect(button.textContent).toContain('Conversation mode off')
    expect(button.className).not.toContain('listening')

    await fireEvent.click(button)

    expect(button.textContent).toContain('Conversation mode on')
    expect(button.className).toContain('listening')

    await fireEvent.click(button)
    expect(button.textContent).toContain('Conversation mode off')
  })

  test('handles unsupported browser', () => {
    mockSpeech.isSupported = false
    const { queryByTitle } = render(VoiceInput, {
      props: {
        onTranscript: vi.fn(),
        onCommand: vi.fn()
      }
    })

    expect(queryByTitle('Start Voice Input')).toBeNull()
  })

  test('passes the lang prop through to the recognition hook', () => {
    render(VoiceInput, {
      props: { onTranscript: vi.fn(), onCommand: vi.fn(), lang: 'pl-PL' }
    })

    expect(speechOptions().lang()).toBe('pl-PL')
  })

  test('forwards recognised transcripts to onTranscript', () => {
    const onTranscript = vi.fn()
    render(VoiceInput, { props: { onTranscript, onCommand: vi.fn() } })

    speechOptions().onTranscript('book a meeting', true)

    expect(onTranscript).toHaveBeenCalledWith('book a meeting')
  })

  test('forwards recognised commands and does not restart when conversation mode is off', async () => {
    vi.useFakeTimers()
    const onCommand = vi.fn()
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand } })

    speechOptions().onCommand('send')

    expect(onCommand).toHaveBeenCalledWith('send')
    vi.advanceTimersByTime(1000)
    expect(mockSpeech.start).not.toHaveBeenCalled()
  })

  test('restarts listening shortly after a command when conversation mode is on', async () => {
    vi.useFakeTimers()
    // Already listening, so the only restart that can be scheduled is the
    // post-command one (the TTS effect only restarts when not listening).
    mockSpeech.isListening = true
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand: vi.fn() } })

    await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
    speechOptions().onCommand('send')

    expect(mockSpeech.start).not.toHaveBeenCalled()
    vi.advanceTimersByTime(200)
    expect(mockSpeech.start).toHaveBeenCalledTimes(1)
  })

  test('does not restart after a command while TTS is speaking', async () => {
    vi.useFakeTimers()
    render(VoiceInput, {
      props: {
        onTranscript: vi.fn(),
        onCommand: vi.fn(),
        ttsSpeaking: true
      }
    })

    await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
    speechOptions().onCommand('send')
    vi.advanceTimersByTime(1000)

    expect(mockSpeech.start).not.toHaveBeenCalled()
  })

  test('logs recognition errors reported by the hook', () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand: vi.fn() } })

    speechOptions().onError('not-allowed')

    expect(errorSpy).toHaveBeenCalledWith('Speech error', 'not-allowed')
    errorSpy.mockRestore()
  })

  test('auto-sends after two seconds of silence in conversation mode', async () => {
    vi.useFakeTimers()
    mockSpeech.isListening = true
    const onCommand = vi.fn()
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand } })

    await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
    speechOptions().onEvent('result')

    vi.advanceTimersByTime(1999)
    expect(onCommand).not.toHaveBeenCalled()

    vi.advanceTimersByTime(1)
    expect(mockSpeech.stop).toHaveBeenCalledTimes(1)
    expect(onCommand).toHaveBeenCalledWith('send')
  })

  test('does not auto-send on silence when conversation mode is off', async () => {
    vi.useFakeTimers()
    mockSpeech.isListening = true
    const onCommand = vi.fn()
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand } })

    speechOptions().onEvent('result')
    vi.advanceTimersByTime(3000)

    expect(onCommand).not.toHaveBeenCalled()
    expect(mockSpeech.stop).not.toHaveBeenCalled()
  })

  test('does not auto-send if listening already stopped before the silence timer fires', async () => {
    vi.useFakeTimers()
    mockSpeech.isListening = true
    const onCommand = vi.fn()
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand } })

    await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
    speechOptions().onEvent('result')
    mockSpeech.isListening = false
    vi.advanceTimersByTime(3000)

    expect(onCommand).not.toHaveBeenCalled()
    expect(mockSpeech.stop).not.toHaveBeenCalled()
  })

  test.each(['end', 'error'])(
    'cancels the pending auto-send on a %s event',
    async (eventType) => {
      vi.useFakeTimers()
      mockSpeech.isListening = true
      const onCommand = vi.fn()
      render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand } })

      await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
      const options = speechOptions()
      options.onEvent('result')
      options.onEvent(eventType)
      vi.advanceTimersByTime(3000)

      expect(onCommand).not.toHaveBeenCalled()
    }
  )

  test('a start event neither schedules nor cancels an auto-send', async () => {
    vi.useFakeTimers()
    mockSpeech.isListening = true
    const onCommand = vi.fn()
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand } })

    await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
    const options = speechOptions()
    options.onEvent('start')
    vi.advanceTimersByTime(3000)
    expect(onCommand).not.toHaveBeenCalled()

    options.onEvent('result')
    options.onEvent('start')
    vi.advanceTimersByTime(2000)
    expect(onCommand).toHaveBeenCalledWith('send')
  })

  test('a command cancels a pending auto-send so it is only sent once', async () => {
    vi.useFakeTimers()
    mockSpeech.isListening = true
    const onCommand = vi.fn()
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand } })

    await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
    const options = speechOptions()
    options.onEvent('result')
    options.onCommand('send')
    vi.advanceTimersByTime(3000)

    expect(onCommand).toHaveBeenCalledTimes(1)
  })

  test('stops listening and drops the pending restart when TTS starts speaking', async () => {
    vi.useFakeTimers()
    mockSpeech.isListening = true
    const props = {
      onTranscript: vi.fn(),
      onCommand: vi.fn(),
      ttsSpeaking: false
    }
    const { rerender } = render(VoiceInput, { props })

    await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
    speechOptions().onCommand('send')

    await rerender({ ...props, ttsSpeaking: true })

    expect(mockSpeech.stop).toHaveBeenCalled()
    vi.advanceTimersByTime(1000)
    expect(mockSpeech.start).not.toHaveBeenCalled()
  })

  test('resumes listening once TTS stops speaking in conversation mode', async () => {
    vi.useFakeTimers()
    const props = {
      onTranscript: vi.fn(),
      onCommand: vi.fn(),
      ttsSpeaking: true
    }
    const { rerender } = render(VoiceInput, { props })

    await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
    expect(mockSpeech.start).not.toHaveBeenCalled()

    await rerender({ ...props, ttsSpeaking: false })
    vi.advanceTimersByTime(200)

    expect(mockSpeech.start).toHaveBeenCalled()
  })

  test('does not resume listening after TTS while the agent is loading', async () => {
    vi.useFakeTimers()
    const props = {
      onTranscript: vi.fn(),
      onCommand: vi.fn(),
      ttsSpeaking: true,
      loading: true
    }
    const { rerender } = render(VoiceInput, { props })

    await fireEvent.click(screen.getByTitle(ALWAYS_ON_TITLE))
    await rerender({ ...props, ttsSpeaking: false })
    vi.advanceTimersByTime(1000)

    expect(mockSpeech.start).not.toHaveBeenCalled()
    expect(screen.getByTitle('Start Voice Input')).toBeDisabled()
  })

  test('space toggles listening when focus is not in a text field', async () => {
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand: vi.fn() } })

    await fireEvent.keyDown(window, { code: 'Space' })

    expect(mockSpeech.toggle).toHaveBeenCalledTimes(1)
  })

  test('space does not toggle listening while typing in an input', async () => {
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand: vi.fn() } })

    const input = document.createElement('input')
    document.body.appendChild(input)
    input.focus()

    await fireEvent.keyDown(window, { code: 'Space' })

    expect(mockSpeech.toggle).not.toHaveBeenCalled()
    input.remove()
  })

  test('space does not toggle listening while focus is in a contenteditable', async () => {
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand: vi.fn() } })

    const editable = document.createElement('div')
    editable.tabIndex = 0
    Object.defineProperty(editable, 'isContentEditable', { value: true })
    document.body.appendChild(editable)
    editable.focus()

    await fireEvent.keyDown(window, { code: 'Space' })

    expect(mockSpeech.toggle).not.toHaveBeenCalled()
    editable.remove()
  })

  test('other keys never toggle listening', async () => {
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand: vi.fn() } })

    await fireEvent.keyDown(window, { code: 'KeyA' })

    expect(mockSpeech.toggle).not.toHaveBeenCalled()
  })

  test('space does not toggle listening when recognition is unsupported', async () => {
    mockSpeech.isSupported = false
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand: vi.fn() } })

    await fireEvent.keyDown(window, { code: 'Space' })

    expect(mockSpeech.toggle).not.toHaveBeenCalled()
  })

  test('surfaces the hook error in the button and a tooltip', () => {
    mockSpeech.error = 'Microphone access denied'
    render(VoiceInput, { props: { onTranscript: vi.fn(), onCommand: vi.fn() } })

    const button = screen.getByTitle('Microphone access denied')
    expect(button.className).toContain('error')
    expect(button.textContent).toContain('Error')
    // Tooltip is rendered outside the button, next to the controls
    const tooltip = document.querySelector('.error-tooltip')
    expect(tooltip?.textContent).toBe('Microphone access denied')
  })
})
