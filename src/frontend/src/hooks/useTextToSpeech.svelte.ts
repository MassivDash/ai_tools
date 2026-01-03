/* eslint-disable no-undef */
export interface TextToSpeechOptions {
  rate?: number
  pitch?: number
  volume?: number
  lang?: string
}

export function useTextToSpeech(options: TextToSpeechOptions = {}) {
  let isSpeaking = $state(false)
  let issupported = $state(false)
  let error = $state<string | null>(null)
  let currentUtteranceId: string | null = null

  // Initialize state based on browser support
  if (typeof window !== 'undefined') {
    issupported = !!window.speechSynthesis
  }

  const { rate = 1, pitch = 1, volume = 1, lang = 'en-US' } = options

  function speak(text: string, langOverride?: string) {
    if (!issupported) {
      error = 'Text to speech not supported'
      return
    }

    // Cancel any current speaking
    cancel()

    // Create a new ID for this utterance to track it
    const utteranceId = Date.now().toString()
    currentUtteranceId = utteranceId
    
    // Set speaking immediately to prevent race conditions
    isSpeaking = true
    error = null

    const utterance = new SpeechSynthesisUtterance(text)
    
    utterance.rate = rate
    utterance.pitch = pitch
    utterance.volume = volume
    utterance.lang = langOverride || lang

    utterance.onstart = () => {
      // Only update if this is still the current utterance
      if (currentUtteranceId === utteranceId) {
        isSpeaking = true
        error = null
      }
    }

    utterance.onend = () => {
      // Only set to false if this was the last requested utterance
      if (currentUtteranceId === utteranceId) {
        isSpeaking = false
      }
    }

    utterance.onerror = (e) => {
      console.error('TTS Error', e)
      if (currentUtteranceId === utteranceId) {
        isSpeaking = false
        error = 'Error speaking text'
      }
    }

    window.speechSynthesis.speak(utterance)
  }

  function cancel() {
    if (issupported && window.speechSynthesis) {
      window.speechSynthesis.cancel()
      isSpeaking = false
      currentUtteranceId = null
    }
  }

  return {
    get isSpeaking() {
      return isSpeaking
    },
    get isSupported() {
      return issupported
    },
    get error() {
      return error
    },
    speak,
    cancel
  }
}
