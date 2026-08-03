/**
 * @vitest-environment jsdom
 */

import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { useTextToSpeech } from './useTextToSpeech.svelte'

// Mock SpeechSynthesisUtterance
class MockSpeechSynthesisUtterance {
  static instances: MockSpeechSynthesisUtterance[] = []

  text: string
  rate = 1
  pitch = 1
  volume = 1
  lang = ''
  onstart: (() => void) | null = null
  onend: (() => void) | null = null
  onerror: ((_event: unknown) => void) | null = null

  constructor(text: string) {
    this.text = text
    MockSpeechSynthesisUtterance.instances.push(this)
  }
}

type SpeechSynthesisMock = {
  speak: ReturnType<typeof vi.fn>
  cancel: ReturnType<typeof vi.fn>
  pause: ReturnType<typeof vi.fn>
  resume: ReturnType<typeof vi.fn>
  getVoices: ReturnType<typeof vi.fn>
  speaking: boolean
  paused: boolean
  onvoiceschanged: (() => void) | null
}

let speechSynthesis: SpeechSynthesisMock

function utterances() {
  return MockSpeechSynthesisUtterance.instances
}

function lastUtterance() {
  const all = utterances()
  return all[all.length - 1]
}

/** Installs a working speechSynthesis + SpeechSynthesisUtterance pair. */
function enableSpeechSynthesis() {
  speechSynthesis = {
    speak: vi.fn(),
    cancel: vi.fn(),
    pause: vi.fn(),
    resume: vi.fn(),
    getVoices: vi.fn(() => []),
    speaking: false,
    paused: false,
    onvoiceschanged: null
  }
  vi.stubGlobal('speechSynthesis', speechSynthesis)
  vi.stubGlobal('SpeechSynthesisUtterance', MockSpeechSynthesisUtterance)
}

beforeEach(() => {
  MockSpeechSynthesisUtterance.instances = []
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

test('reports unsupported when speechSynthesis is missing', () => {
  const tts = useTextToSpeech()

  expect(tts.isSupported).toBe(false)
  expect(tts.isSpeaking).toBe(false)
  expect(tts.error).toBe(null)
})

test('speak() sets an error and does nothing when unsupported', () => {
  vi.stubGlobal('SpeechSynthesisUtterance', MockSpeechSynthesisUtterance)
  const tts = useTextToSpeech()

  tts.speak('hello there')

  expect(tts.error).toBe('Text to speech not supported')
  expect(tts.isSpeaking).toBe(false)
  expect(utterances()).toHaveLength(0)
})

test('reports unsupported when there is no window at all (SSR)', () => {
  enableSpeechSynthesis()
  vi.stubGlobal('window', undefined)

  const tts = useTextToSpeech()

  expect(tts.isSupported).toBe(false)
  tts.speak('nobody is listening')
  expect(tts.error).toBe('Text to speech not supported')
  expect(utterances()).toHaveLength(0)
})

test('cancel() is a no-op when unsupported', () => {
  const tts = useTextToSpeech()

  expect(() => tts.cancel()).not.toThrow()
  expect(tts.isSpeaking).toBe(false)
})

test('reports supported when speechSynthesis is present', () => {
  enableSpeechSynthesis()

  const tts = useTextToSpeech()

  expect(tts.isSupported).toBe(true)
})

test('speak() cancels any previous speech, then queues an utterance', () => {
  enableSpeechSynthesis()
  const tts = useTextToSpeech()

  tts.speak('hello world')

  expect(speechSynthesis.cancel).toHaveBeenCalledTimes(1)
  expect(speechSynthesis.speak).toHaveBeenCalledTimes(1)
  expect(speechSynthesis.speak).toHaveBeenCalledWith(lastUtterance())
  expect(lastUtterance().text).toBe('hello world')
  // Speaking flips immediately, before the browser fires onstart.
  expect(tts.isSpeaking).toBe(true)
  expect(tts.error).toBe(null)
})

test('applies default rate / pitch / volume / lang', () => {
  enableSpeechSynthesis()
  const tts = useTextToSpeech()

  tts.speak('defaults')

  expect(lastUtterance()).toMatchObject({
    rate: 1,
    pitch: 1,
    volume: 1,
    lang: 'en-US'
  })
})

test('applies the configured rate / pitch / volume / lang', () => {
  enableSpeechSynthesis()
  const tts = useTextToSpeech({
    rate: 1.75,
    pitch: 0.5,
    volume: 0.25,
    lang: 'pl-PL'
  })

  tts.speak('konfiguracja')

  expect(lastUtterance()).toMatchObject({
    rate: 1.75,
    pitch: 0.5,
    volume: 0.25,
    lang: 'pl-PL'
  })
})

test('a per-call language overrides the configured one', () => {
  enableSpeechSynthesis()
  const tts = useTextToSpeech({ lang: 'en-US' })

  tts.speak('bonjour', 'fr-FR')

  expect(lastUtterance().lang).toBe('fr-FR')
})

test('onstart keeps the hook in the speaking state and clears errors', () => {
  enableSpeechSynthesis()
  const tts = useTextToSpeech()

  tts.speak('first')
  lastUtterance().onerror?.({ error: 'canceled' })
  expect(tts.error).toBe('Error speaking text')

  tts.speak('second')
  lastUtterance().onstart?.()

  expect(tts.isSpeaking).toBe(true)
  expect(tts.error).toBe(null)
})

test('onend clears the speaking state', () => {
  enableSpeechSynthesis()
  const tts = useTextToSpeech()

  tts.speak('finish me')
  expect(tts.isSpeaking).toBe(true)

  lastUtterance().onend?.()

  expect(tts.isSpeaking).toBe(false)
  expect(tts.error).toBe(null)
})

test('onerror clears the speaking state and records an error', () => {
  enableSpeechSynthesis()
  const tts = useTextToSpeech()

  tts.speak('boom')
  lastUtterance().onerror?.({ error: 'synthesis-failed' })

  expect(tts.isSpeaking).toBe(false)
  expect(tts.error).toBe('Error speaking text')
  expect(console.error).toHaveBeenCalled()
})

test('callbacks from a superseded utterance are ignored', () => {
  enableSpeechSynthesis()
  // Distinct Date.now() values so the two utterances get distinct ids.
  vi.spyOn(Date, 'now').mockReturnValueOnce(1000).mockReturnValueOnce(2000)
  const tts = useTextToSpeech()

  tts.speak('stale')
  const stale = lastUtterance()
  tts.speak('current')
  const current = lastUtterance()

  expect(stale).not.toBe(current)
  expect(tts.isSpeaking).toBe(true)

  // The stale utterance's late callbacks must not clobber the current state.
  stale.onend?.()
  expect(tts.isSpeaking).toBe(true)

  stale.onerror?.({ error: 'interrupted' })
  expect(tts.isSpeaking).toBe(true)
  expect(tts.error).toBe(null)

  stale.onstart?.()
  expect(tts.isSpeaking).toBe(true)

  // The current utterance is still in charge.
  current.onend?.()
  expect(tts.isSpeaking).toBe(false)
})

test('cancel() stops the synthesiser and clears the speaking state', () => {
  enableSpeechSynthesis()
  const tts = useTextToSpeech()

  tts.speak('interrupt me')
  expect(tts.isSpeaking).toBe(true)
  speechSynthesis.cancel.mockClear()

  tts.cancel()

  expect(speechSynthesis.cancel).toHaveBeenCalledTimes(1)
  expect(tts.isSpeaking).toBe(false)
})

test('after cancel(), the cancelled utterance can no longer change state', () => {
  enableSpeechSynthesis()
  const tts = useTextToSpeech()

  tts.speak('cancelled')
  const utterance = lastUtterance()
  tts.cancel()

  utterance.onstart?.()

  expect(tts.isSpeaking).toBe(false)
})
