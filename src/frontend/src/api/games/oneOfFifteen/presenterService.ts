import { axiosBackendInstance } from '@axios/axiosBackendInstance'
import type { GameStateSnapshot } from '../../../hooks/useOneOfFifteenState.svelte'

export const generateIntroSpeech = async (
  contestants: GameStateSnapshot['contestants']
): Promise<string> => {
  try {
    const names = contestants.map((c) => c.name).join(', ')
    const count = contestants.length
    const prompt = `Welcome to One of 15. There are ${count} contestants: ${names}. Say hello to them, tell a joke, and end with "Let's go!!". Keep it under 40 words.`

    const systemPrompt = `You are a quirky, slightly emotional Robot Quiz Host named Quiz Bot.`

    const requestPayload = {
      message: `${systemPrompt} ${prompt}`,
      conversation_id: undefined
    }

    /* eslint-disable no-undef */
    const response = await fetch(
      `${axiosBackendInstance.defaults.baseURL}/agent/chat/stream`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestPayload)
      }
    )
    /* eslint-enable no-undef */

    if (!response.body) return "Welcome everyone! Let's go!!"

    const reader = response.body.getReader()
    /* eslint-disable no-undef */
    const decoder = new TextDecoder()
    /* eslint-enable no-undef */
    let fullText = ''

    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      const chunk = decoder.decode(value)
      const lines = chunk.split('\n')
      for (const line of lines) {
        if (line.startsWith('data: ')) {
          try {
            const data = JSON.parse(line.slice(6))
            if (data.type === 'text_chunk' && data.text) {
              fullText += data.text
            }
          } catch {
            // ignore
          }
        }
      }
    }
    return fullText.trim() || "Welcome quite everyone! Let's go!!"
  } catch (e) {
    console.error('LLM Error', e)
    return "Welcome to the game! Let's start!"
  }
}
