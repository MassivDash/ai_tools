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
  
  // Initialize state based on browser support
  if (typeof window !== 'undefined') {
    issupported = !!window.speechSynthesis
  }

  const {
    rate = 1,
    pitch = 1,
    volume = 1,
    lang = 'en-US'
  } = options

  function speak(text: string) {
    if (!issupported) {
        error = "Text to speech not supported"
        return
    }

    // Cancel any current speaking
    cancel()

    const utterance = new SpeechSynthesisUtterance(text)
    utterance.rate = rate
    utterance.pitch = pitch
    utterance.volume = volume
    utterance.lang = lang

    utterance.onstart = () => {
      isSpeaking = true
      error = null
    }

    utterance.onend = () => {
      isSpeaking = false
    }

    utterance.onerror = (e) => {
      console.error("TTS Error", e)
      isSpeaking = false
      error = "Error speaking text"
    }

    window.speechSynthesis.speak(utterance)
  }

  function cancel() {
    if (issupported && window.speechSynthesis) {
        window.speechSynthesis.cancel()
        isSpeaking = false
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
