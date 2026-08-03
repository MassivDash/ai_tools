/**
 * @vitest-environment jsdom
 */

import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { useGameSession } from './useGameSession'

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/

beforeEach(() => {
  window.sessionStorage.clear()
})

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

test('generates and persists a session id under a game-scoped key', () => {
  const { sessionId } = useGameSession('one-of-ten')

  expect(sessionId).toBeTruthy()
  expect(window.sessionStorage.getItem('game_session_one-of-ten')).toBe(
    sessionId
  )
})

test('reuses an already-stored session id instead of generating a new one', () => {
  window.sessionStorage.setItem('game_session_quiz', 'existing-session-id')
  const randomUUID = vi.spyOn(window.crypto, 'randomUUID')

  const { sessionId } = useGameSession('quiz')

  expect(sessionId).toBe('existing-session-id')
  expect(randomUUID).not.toHaveBeenCalled()
})

test('different game ids get independent sessions', () => {
  const a = useGameSession('game-a')
  const b = useGameSession('game-b')

  expect(a.sessionId).not.toBe(b.sessionId)
  expect(window.sessionStorage.getItem('game_session_game-a')).toBe(a.sessionId)
  expect(window.sessionStorage.getItem('game_session_game-b')).toBe(b.sessionId)
})

test('uses crypto.randomUUID when it is available', () => {
  const randomUUID = vi
    .spyOn(window.crypto, 'randomUUID')
    .mockReturnValue('11111111-2222-4333-8444-555555555555')

  const { sessionId } = useGameSession('crypto-game')

  expect(randomUUID).toHaveBeenCalled()
  expect(sessionId).toBe('11111111-2222-4333-8444-555555555555')
})

test('falls back to a hand-built v4 uuid when crypto.randomUUID is missing', () => {
  // Non-secure contexts (plain HTTP) do not expose randomUUID.
  vi.stubGlobal('crypto', {})

  const { sessionId } = useGameSession('insecure-game')

  expect(sessionId).toMatch(UUID_RE)
  expect(window.sessionStorage.getItem('game_session_insecure-game')).toBe(
    sessionId
  )
})

test('the fallback generator produces distinct ids', () => {
  vi.stubGlobal('crypto', {})

  const ids = new Set<string>()
  for (let i = 0; i < 20; i++) {
    window.sessionStorage.clear()
    ids.add(useGameSession('g').sessionId as string)
  }

  expect(ids.size).toBe(20)
  ids.forEach((id) => expect(id).toMatch(UUID_RE))
})

test('falls back when crypto itself is absent', () => {
  vi.stubGlobal('crypto', undefined)

  const { sessionId } = useGameSession('no-crypto')

  expect(sessionId).toMatch(UUID_RE)
})

test('clearSession removes only that game key', () => {
  const a = useGameSession('game-a')
  useGameSession('game-b')

  a.clearSession()

  expect(window.sessionStorage.getItem('game_session_game-a')).toBe(null)
  expect(window.sessionStorage.getItem('game_session_game-b')).not.toBe(null)
})

test('a new session is issued after clearSession', () => {
  const first = useGameSession('rejoin')
  first.clearSession()

  const second = useGameSession('rejoin')

  expect(second.sessionId).toBeTruthy()
  expect(second.sessionId).not.toBe(first.sessionId)
})
