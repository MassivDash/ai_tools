import { axiosBackendInstance } from '@axios/axiosBackendInstance'
import type { GameStateSnapshot } from '../../../hooks/useOneOfTenState.svelte'

const QUIZ_BOT_SYSTEM_PROMPT =
  'You are a quirky, slightly emotional Robot Quiz Host named Quiz Bot.'

const streamHostLine = async (
  prompt: string,
  fallback: string
): Promise<string> => {
  try {
    const requestPayload = {
      message: `${QUIZ_BOT_SYSTEM_PROMPT} ${prompt}`,
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

    if (!response.body) return fallback

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
    return fullText.trim() || fallback
  } catch (e) {
    console.error('LLM Error', e)
    return fallback
  }
}

export const generateIntroSpeech = async (
  contestants: GameStateSnapshot['contestants']
): Promise<string> => {
  const names = contestants.map((c) => c.name).join(', ')
  const count = contestants.length
  const prompt = `Welcome to 1 z 10. There are ${count} contestants: ${names}. Say hello to them and end with "Let's go!!". Keep it under 40 words.`

  return streamHostLine(prompt, "Welcome everyone! Let's go!!")
}

export const generateHostJoke = async (): Promise<string> => {
  const prompt =
    'Tell one short, corny, not-very-funny dad joke to open the game show. Just the joke itself, under 20 words, no preamble.'

  return streamHostLine(
    prompt,
    'Why did the quiz show host bring a ladder? To reach the high scores!'
  )
}

export const generateAnswerComment = async (
  question: string,
  isCorrect: boolean,
  correctAnswer?: string | null
): Promise<string> => {
  const prompt = isCorrect
    ? `A contestant just correctly answered this question: "${question}". React with a short, enthusiastic host comment, under 20 words. If the question is fairly obscure or difficult, you may add one brief interesting extra detail about the answer.`
    : `A contestant answered this question incorrectly: "${question}". The correct answer is "${correctAnswer}". React with a short, in-character host comment that reveals the correct answer, under 20 words.`

  const fallback = isCorrect
    ? "That's correct!"
    : `Sorry, that's incorrect. The correct answer was: ${correctAnswer}.`

  return streamHostLine(prompt, fallback)
}

export const generateWinnerSpeech = async (
  winnerName: string,
  winnerScore: number
): Promise<string> => {
  const prompt = `The game has ended, and the winner is ${winnerName} with a score of ${winnerScore} points. Announce the winner in your quirky, robot host persona and say a little congratulations comment. Keep it under 40 words.`
  const fallback = `Congratulations to ${winnerName}! You are the winner with ${winnerScore} points!`

  return streamHostLine(prompt, fallback)
}
