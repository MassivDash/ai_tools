/* eslint-disable no-undef */
export interface SpeechRecognitionOptions {
  onTranscript: (_transcript: string, _isFinal: boolean) => void
  onCommand?: (_command: 'execute' | 'send') => void
  onError?: (_error: string) => void
  onEvent?: (_eventType: 'start' | 'end' | 'result' | 'error') => void
  lang?: string | (() => string)
}

export function useSpeechRecognition({
  onTranscript,
  onCommand,
  onError,
  onEvent,
  lang = 'en-US'
}: SpeechRecognitionOptions) {
  let isListening = $state(false)
  let recognition: any = $state(null)
  let error = $state<string | null>(null)

  // Check support immediately
  const isSupported =
    typeof window !== 'undefined' &&
    (!!(window as any).SpeechRecognition ||
      !!(window as any).webkitSpeechRecognition)

  function toggle() {
    if (isListening) {
      stop()
      return
    }
    start()
  }

  function start() {
    error = null

    if (!isSupported) {
      error = 'Speech recognition not supported'
      return
    }

    if (!recognition) {
      const SpeechRecognition =
        (window as any).SpeechRecognition ||
        (window as any).webkitSpeechRecognition

      recognition = new SpeechRecognition()
      recognition.continuous = true
      recognition.interimResults = true
      recognition.lang = typeof lang === 'function' ? lang() : lang

      recognition.onstart = () => {
        isListening = true
        error = null
      }

      recognition.onend = () => {
        isListening = false
      }

      recognition.onerror = (event: any) => {
        console.error('Speech recognition error', event.error)
        isListening = false
        if (event.error === 'network') {
          error = 'Network error: Check connection'
        } else if (event.error === 'not-allowed') {
          error = 'Microphone access denied'
        } else if (event.error === 'no-speech') {
          // Ignore no-speech errors (common when silence)
          return
        } else {
          error = `Error: ${event.error}`
        }
        onError?.(event.error)
      }

      recognition.onresult = (event: any) => {
        onEvent?.('result')

        let finalTranscript = ''

        for (let i = event.resultIndex; i < event.results.length; ++i) {
          if (event.results[i].isFinal) {
            finalTranscript += event.results[i][0].transcript
          }
        }

        if (finalTranscript) {
          const lowerTranscript = finalTranscript.toLowerCase().trim()
          if (
            lowerTranscript.endsWith('execute') ||
            lowerTranscript.endsWith('send')
          ) {
            const cleanTranscript = finalTranscript
              .replace(/execute$/i, '')
              .replace(/send$/i, '')
              .trim()

            // Notify complete transcript without command
            onTranscript(cleanTranscript, true)

            // Trigger command
            stop()
            onCommand?.('send')
          } else {
            onTranscript(finalTranscript, true)
          }
        }
      }
    }

    try {
      recognition.start()
    } catch (e) {
      console.error('Failed to start recognition', e)
      error = 'Failed to start'
    }
  }

  function stop() {
    if (recognition) {
      recognition.stop()
      isListening = false
    }
  }

  $effect(() => {
    const currentLang = typeof lang === 'function' ? lang() : lang
    if (recognition && recognition.lang !== currentLang) {
      const wasListening = isListening
      if (wasListening) recognition.stop()
      recognition.lang = currentLang
      if (wasListening) {
        // slight delay to allow stop to process
        setTimeout(() => recognition.start(), 100)
      }
    }
  })

  return {
    get isListening() {
      return isListening
    },
    get error() {
      return error
    },
    get isSupported() {
      return isSupported
    },
    toggle,
    start,
    stop
  }
}
