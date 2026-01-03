export function useGameSession(gameId: string) {
  const storageKey = `game_session_${gameId}`

  // Check for existing session or generate new one
  let sessionId: string | null = null

  if (typeof window !== 'undefined') {
    sessionId = window.sessionStorage.getItem(storageKey)
    if (!sessionId) {
      sessionId = window.crypto.randomUUID()
      window.sessionStorage.setItem(storageKey, sessionId)
    }
  }

  const clearSession = () => {
    if (typeof window !== 'undefined') {
      window.sessionStorage.removeItem(storageKey)
    }
  }

  return {
    sessionId,
    clearSession
  }
}
