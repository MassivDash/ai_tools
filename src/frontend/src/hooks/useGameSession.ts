export function useGameSession(gameId: string) {
  const storageKey = `game_session_${gameId}`

  // Check for existing session or generate new one
  let sessionId: string | null = null

  if (typeof window !== 'undefined') {
    sessionId = window.sessionStorage.getItem(storageKey)
    if (!sessionId) {
      if (window.crypto && window.crypto.randomUUID) {
        sessionId = window.crypto.randomUUID()
      } else {
        // Fallback for non-secure contexts (HTTP)
        sessionId = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(
          /[xy]/g,
          function (c) {
            const r = (Math.random() * 16) | 0
            const v = c === 'x' ? r : (r & 0x3) | 0x8
            return v.toString(16)
          }
        )
      }
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
