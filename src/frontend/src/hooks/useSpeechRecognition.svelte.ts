/* eslint-disable no-undef */
export interface SpeechRecognitionOptions {
  onTranscript: (transcript: string, isFinal: boolean) => void
  onCommand?: (command: 'execute' | 'send') => void
  onError?: (error: any) => void
}

export function useSpeechRecognition({
  onTranscript,
  onCommand,
  onError
}: SpeechRecognitionOptions) {
  let isListening = $state(false)
  let recognition: any = $state(null)

  let error = $state<string | null>(null)

  function toggle() {
    if (isListening) {
      stop()
      return
    }
    start()
  }

  function start() {
    error = null
    if (!recognition) {
      const SpeechRecognition =
        (window as any).SpeechRecognition ||
        (window as any).webkitSpeechRecognition

      if (!SpeechRecognition) {
        error = 'Speech recognition not supported'
        alert('Speech recognition is not supported in this browser.')
        return
      }

      recognition = new SpeechRecognition()
      recognition.continuous = true
      recognition.interimResults = true
      recognition.lang = 'en-US'

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

  return {
    get isListening() {
      return isListening
    },
    get error() {
      return error
    },
    toggle,
    start,
    stop
  }
}
