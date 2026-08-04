/**
 * @vitest-environment jsdom
 *
 * Named `*.svelte.test.ts` so vite-plugin-svelte compiles the runes in this
 * file: `useSpeechRecognition` registers an `$effect`, and an `$effect` needs an
 * `$effect.root` to live in.
 *
 * Note: in the node/ssr vitest project `svelte` resolves to its server build,
 * where `flushSync` is a no-op — effects are flushed by awaiting a task instead
 * (see `settle`).
 */

import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { useSpeechRecognition } from './useSpeechRecognition.svelte'

// Mock SpeechRecognition
class MockSpeechRecognition {
  static instances: MockSpeechRecognition[] = []

  continuous = false
  interimResults = false
  lang = ''
  startCalls = 0
  stopCalls = 0
  throwOnStart: Error | null = null

  onstart: (() => void) | null = null
  onend: (() => void) | null = null
  onerror: ((_event: { error: string }) => void) | null = null
  onresult: ((_event: SpeechResultEvent) => void) | null = null

  constructor() {
    MockSpeechRecognition.instances.push(this)
  }

  start() {
    this.startCalls++
    if (this.throwOnStart) throw this.throwOnStart
    this.onstart?.()
  }

  stop() {
    this.stopCalls++
    this.onend?.()
  }
}

type SpeechResultEvent = {
  resultIndex: number
  results: Array<{ 0: { transcript: string }; isFinal: boolean }>
}

/** Builds the array-of-array-likes shape the browser hands to `onresult`. */
function resultEvent(
  entries: Array<[transcript: string, isFinal: boolean]>,
  resultIndex = 0
): SpeechResultEvent {
  return {
    resultIndex,
    results: entries.map(([transcript, isFinal]) => ({
      0: { transcript },
      isFinal
    }))
  }
}

/** Lets Svelte flush its pending effects. */
function settle() {
  return new Promise((resolve) => setTimeout(resolve, 0))
}

function only() {
  expect(MockSpeechRecognition.instances).toHaveLength(1)
  return MockSpeechRecognition.instances[0]
}

const cleanups: Array<() => void> = []

/** Runs the hook inside an effect root and flushes its initial effect. */
async function mount(
  options: Parameters<typeof useSpeechRecognition>[0]
): Promise<ReturnType<typeof useSpeechRecognition>> {
  let hook!: ReturnType<typeof useSpeechRecognition>
  cleanups.push(
    $effect.root(() => {
      hook = useSpeechRecognition(options)
    })
  )
  await settle()
  return hook
}

beforeEach(() => {
  MockSpeechRecognition.instances = []
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  while (cleanups.length) cleanups.pop()!()
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

// ---------------------------------------------------------------- unsupported

test('reports unsupported when neither global exists', async () => {
  const sr = await mount({ onTranscript: vi.fn() })

  expect(sr.isSupported).toBe(false)
  expect(sr.isListening).toBe(false)
  expect(sr.error).toBe(null)
})

test('start() sets an error and creates nothing when unsupported', async () => {
  const onTranscript = vi.fn()
  const sr = await mount({ onTranscript })

  sr.start()

  expect(sr.error).toBe('Speech recognition not supported')
  expect(sr.isListening).toBe(false)
  expect(MockSpeechRecognition.instances).toHaveLength(0)
  expect(onTranscript).not.toHaveBeenCalled()
})

test('toggle() surfaces the unsupported error too', async () => {
  const sr = await mount({ onTranscript: vi.fn() })

  sr.toggle()

  expect(sr.error).toBe('Speech recognition not supported')
})

test('stop() is a no-op before anything was started', async () => {
  const sr = await mount({ onTranscript: vi.fn() })

  expect(() => sr.stop()).not.toThrow()
  expect(sr.isListening).toBe(false)
})

// ------------------------------------------------------------------ supported

test('detects support via the unprefixed global and configures recognition', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn() })

  expect(sr.isSupported).toBe(true)

  sr.start()

  const recognition = only()
  expect(recognition.continuous).toBe(true)
  expect(recognition.interimResults).toBe(true)
  expect(recognition.lang).toBe('en-US')
  expect(recognition.startCalls).toBe(1)
  expect(sr.isListening).toBe(true)
  expect(sr.error).toBe(null)
})

test('detects support via the webkit-prefixed global', async () => {
  vi.stubGlobal('webkitSpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn() })

  expect(sr.isSupported).toBe(true)

  sr.start()

  expect(only().startCalls).toBe(1)
  expect(sr.isListening).toBe(true)
})

test('uses a static lang option', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn(), lang: 'de-DE' })

  sr.start()

  expect(only().lang).toBe('de-DE')
})

test('resolves a lang getter at start time', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn(), lang: () => 'pl-PL' })

  sr.start()

  expect(only().lang).toBe('pl-PL')
})

test('reuses the same recognition object across start/stop cycles', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn() })

  sr.start()
  sr.stop()
  sr.start()

  const recognition = only()
  expect(recognition.startCalls).toBe(2)
  expect(recognition.stopCalls).toBe(1)
  expect(sr.isListening).toBe(true)
})

test('toggle() starts, then stops', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn() })

  sr.toggle()
  expect(sr.isListening).toBe(true)

  sr.toggle()

  expect(sr.isListening).toBe(false)
  expect(only().stopCalls).toBe(1)
})

test('records an error when recognition.start() throws', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn() })

  // The first start creates the instance; arm it to throw, then retry.
  sr.start()
  sr.stop()
  only().throwOnStart = new Error('already started')

  sr.start()

  expect(sr.error).toBe('Failed to start')
  expect(sr.isListening).toBe(false)
  expect(console.error).toHaveBeenCalled()
})

test('onend clears the listening state', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn() })

  sr.start()
  expect(sr.isListening).toBe(true)

  only().onend?.()

  expect(sr.isListening).toBe(false)
})

// -------------------------------------------------------------------- results

test('forwards a final transcript and reports the result event', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onTranscript = vi.fn()
  const onEvent = vi.fn()
  const sr = await mount({ onTranscript, onEvent })

  sr.start()
  only().onresult?.(resultEvent([['hello world', true]]))

  expect(onEvent).toHaveBeenCalledWith('result')
  expect(onTranscript).toHaveBeenCalledTimes(1)
  expect(onTranscript).toHaveBeenCalledWith('hello world', true)
})

test('ignores interim results but still reports the event', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onTranscript = vi.fn()
  const onEvent = vi.fn()
  const sr = await mount({ onTranscript, onEvent })

  sr.start()
  only().onresult?.(resultEvent([['partial thou', false]]))

  expect(onEvent).toHaveBeenCalledWith('result')
  expect(onTranscript).not.toHaveBeenCalled()
})

test('concatenates the final alternatives from resultIndex onwards', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onTranscript = vi.fn()
  const sr = await mount({ onTranscript })

  sr.start()
  only().onresult?.(
    resultEvent(
      [
        ['already delivered ', true],
        ['brand ', true],
        ['still typing', false],
        ['new', true]
      ],
      1
    )
  )

  expect(onTranscript).toHaveBeenCalledWith('brand new', true)
})

test('strips a trailing "execute" command and triggers it', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onTranscript = vi.fn()
  const onCommand = vi.fn()
  const sr = await mount({ onTranscript, onCommand })

  sr.start()
  only().onresult?.(resultEvent([['list the files execute', true]]))

  expect(onTranscript).toHaveBeenCalledWith('list the files', true)
  expect(onCommand).toHaveBeenCalledWith('send')
  expect(only().stopCalls).toBe(1)
  expect(sr.isListening).toBe(false)
})

test('strips a trailing "send" command case-insensitively and triggers it', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onTranscript = vi.fn()
  const onCommand = vi.fn()
  const sr = await mount({ onTranscript, onCommand })

  sr.start()
  only().onresult?.(resultEvent([['what is the weather Send', true]]))

  expect(onTranscript).toHaveBeenCalledWith('what is the weather', true)
  expect(onCommand).toHaveBeenCalledWith('send')
})

test('a command transcript works without an onCommand callback', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onTranscript = vi.fn()
  const sr = await mount({ onTranscript })

  sr.start()
  expect(() =>
    only().onresult?.(resultEvent([['do the thing execute', true]]))
  ).not.toThrow()

  expect(onTranscript).toHaveBeenCalledWith('do the thing', true)
  expect(sr.isListening).toBe(false)
})

test('a transcript that merely contains "send" is left intact', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onTranscript = vi.fn()
  const onCommand = vi.fn()
  const sr = await mount({ onTranscript, onCommand })

  sr.start()
  only().onresult?.(resultEvent([['send me the report', true]]))

  expect(onTranscript).toHaveBeenCalledWith('send me the report', true)
  expect(onCommand).not.toHaveBeenCalled()
  expect(sr.isListening).toBe(true)
})

test('results without an onEvent callback are handled', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onTranscript = vi.fn()
  const sr = await mount({ onTranscript })

  sr.start()
  expect(() =>
    only().onresult?.(resultEvent([['no event listener', true]]))
  ).not.toThrow()

  expect(onTranscript).toHaveBeenCalledWith('no event listener', true)
})

// --------------------------------------------------------------------- errors

test('maps a network error to a friendly message', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onError = vi.fn()
  const sr = await mount({ onTranscript: vi.fn(), onError })

  sr.start()
  only().onerror?.({ error: 'network' })

  expect(sr.error).toBe('Network error: Check connection')
  expect(sr.isListening).toBe(false)
  expect(onError).toHaveBeenCalledWith('network')
})

test('maps a permission error to a friendly message', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onError = vi.fn()
  const sr = await mount({ onTranscript: vi.fn(), onError })

  sr.start()
  only().onerror?.({ error: 'not-allowed' })

  expect(sr.error).toBe('Microphone access denied')
  expect(onError).toHaveBeenCalledWith('not-allowed')
})

test('swallows no-speech errors without reporting them', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onError = vi.fn()
  const sr = await mount({ onTranscript: vi.fn(), onError })

  sr.start()
  only().onerror?.({ error: 'no-speech' })

  expect(sr.error).toBe(null)
  expect(onError).not.toHaveBeenCalled()
  // The listening flag is still cleared before the early return.
  expect(sr.isListening).toBe(false)
})

test('falls back to a generic message for unknown errors', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const onError = vi.fn()
  const sr = await mount({ onTranscript: vi.fn(), onError })

  sr.start()
  only().onerror?.({ error: 'audio-capture' })

  expect(sr.error).toBe('Error: audio-capture')
  expect(onError).toHaveBeenCalledWith('audio-capture')
})

test('errors are tolerated without an onError callback', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn() })

  sr.start()
  expect(() => only().onerror?.({ error: 'aborted' })).not.toThrow()

  expect(sr.error).toBe('Error: aborted')
})

test('restarting clears a previous error', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const sr = await mount({ onTranscript: vi.fn() })

  sr.start()
  only().onerror?.({ error: 'network' })
  expect(sr.error).toBe('Network error: Check connection')

  sr.start()

  expect(sr.error).toBe(null)
  expect(sr.isListening).toBe(true)
})

// ---------------------------------------------------------- reactive language

test('an unchanged lang leaves recognition alone when the effect re-runs', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const config = $state({ lang: 'en-US' })
  const sr = await mount({ onTranscript: vi.fn(), lang: () => config.lang })

  sr.start()
  // Assigning `recognition` invalidates the effect; it must then no-op.
  await settle()

  expect(only().lang).toBe('en-US')
  expect(only().stopCalls).toBe(0)
  expect(only().startCalls).toBe(1)
  expect(sr.isListening).toBe(true)
})

test('a reactive lang change while idle just updates recognition.lang', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const config = $state({ lang: 'en-US' })
  const sr = await mount({ onTranscript: vi.fn(), lang: () => config.lang })

  sr.start()
  sr.stop()
  expect(only().startCalls).toBe(1)

  config.lang = 'es-ES'
  await settle()

  expect(only().lang).toBe('es-ES')
  expect(only().startCalls).toBe(1)
  expect(only().stopCalls).toBe(1)
  expect(sr.isListening).toBe(false)
})

test('a reactive lang change while listening restarts recognition', async () => {
  vi.stubGlobal('SpeechRecognition', MockSpeechRecognition)
  const config = $state({ lang: 'en-US' })
  const sr = await mount({ onTranscript: vi.fn(), lang: () => config.lang })

  sr.start()
  expect(sr.isListening).toBe(true)

  config.lang = 'fr-FR'
  await settle()

  const recognition = only()
  expect(recognition.lang).toBe('fr-FR')
  expect(recognition.stopCalls).toBe(1)
  // The restart is deferred by 100ms.
  expect(recognition.startCalls).toBe(1)

  await new Promise((resolve) => setTimeout(resolve, 150))

  expect(recognition.startCalls).toBe(2)
  expect(sr.isListening).toBe(true)
})
