import { useTextToSpeech } from './useTextToSpeech.svelte'

export function usePresenterSpeech() {
  const tts = useTextToSpeech()
  let robotTalking = $state(false)

  const speakAndWait = async (text: string) => {
    robotTalking = true
    tts.speak(text)
    // Wait for start
    await new Promise((r) => setTimeout(r, 100))
    // Poll loop
    while (tts.isSpeaking) {
      await new Promise((r) => setTimeout(r, 200))
    }
    robotTalking = false
  }

  return {
    get robotTalking() {
      return robotTalking
    },
    speakAndWait,
    speak: tts.speak,
    cancel: tts.cancel,
    get isSpeaking() {
      return tts.isSpeaking
    }
  }
}
