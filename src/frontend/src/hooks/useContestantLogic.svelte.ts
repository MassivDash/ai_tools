import type { GameStateSnapshot } from './useOneOfTenState.svelte'

// No imports needed for runes in .svelte.ts files in Svelte 5?
// Actually, in .svelte.ts files, runes are available globally if configured correctly,
// BUT this is a .ts file. Runes are only for .svelte files or .svelte.ts/js.
// Wait, the file extension IS .svelte.ts.
// However, the linter says '$derived is not defined'.
// It might be a linter configuration issue or we need to respect the environment.
// Let's check if we need to disable the linter or if we are missing something.
// Actually, standard eslint might not know about runes in .svelte.ts files yet.
// We can try to ignore the linter rule for this file or assume it's valid Svelte 5.
// But wait, the previous errors were from the linter check *after* the file creation.
// Ah, the errors are: "$derived is not defined".
// If I look at `useTextToSpeech.svelte.ts`, it has `/* eslint-disable no-undef */`.
// I will apply the same fix.

export const PHASE = {
  LOBBY: 'LOBBY',
  ELIMINATED: 'ELIMINATED',
  POINTING: 'POINTING',
  BUZZER: 'BUZZER',
  DECISION: 'DECISION',
  SPECTATING_DECISION: 'SPECTATING_DECISION',
  ANSWERING: 'ANSWERING',
  WAITING: 'WAITING',
  WAITING_FOR_PRESENTER: 'WAITING_FOR_PRESENTER',
  FINISHED: 'FINISHED'
} as const

export function useContestantLogic(
  getGameState: () => GameStateSnapshot,
  sessionId: string
) {
  // Derived Player State
  const myContestant = $derived(
    getGameState().contestants.find(
      (c) => c.id === sessionId || c.session_id === sessionId
    )
  )

  const isReady = $derived(myContestant?.ready || false)
  const isActivePlayer = $derived(getGameState().active_player_id === sessionId)
  const isEliminated = $derived(myContestant?.eliminated || false)

  // Derived Round State
  const isRound1 = $derived(getGameState().round === 'round1')
  const isRound2 = $derived(getGameState().round === 'round2')
  const isRound3 = $derived(getGameState().round === 'round3')

  // Phase Logic
  const isPointingPhase = $derived(isRound2 && !getGameState().current_question)
  const isMyTurnToPoint = $derived(isActivePlayer && isPointingPhase)
  const isBuzzerPhase = $derived(isRound3 && !getGameState().active_player_id)
  const isDecisionPhase = $derived(
    isRound3 && isActivePlayer && getGameState().decision_pending
  )

  // Names
  const activePlayerName = $derived(
    getGameState().contestants.find(
      (c) => c.id === getGameState().active_player_id
    )?.name ||
      getGameState().active_player_id ||
      'Unknown'
  )
  const pointerName = $derived(activePlayerName) // Alias for clarity
  const deciderName = $derived(activePlayerName) // Alias for clarity

  // Master Phase Selector
  const currentPhase = $derived.by(() => {
    const gs = getGameState()
    if (gs.round === 'lobby') return PHASE.LOBBY
    if (isEliminated) return PHASE.ELIMINATED
    if (gs.round === 'finished') return PHASE.FINISHED
    if (gs.waiting_for_presenter) return PHASE.WAITING_FOR_PRESENTER

    if (isRound2 && isPointingPhase) return PHASE.POINTING
    if (isRound3) {
      if (isBuzzerPhase) return PHASE.BUZZER
      if (isDecisionPhase) return PHASE.DECISION
      if (gs.decision_pending && !isActivePlayer)
        return PHASE.SPECTATING_DECISION
    }

    if (isActivePlayer) return PHASE.ANSWERING

    return PHASE.WAITING
  })

  // Status Message Logic
  const statusMessage = $derived.by(() => {
    switch (currentPhase) {
      case PHASE.LOBBY:
        return 'Waiting for game to start...'
      case PHASE.ELIMINATED:
        return 'ELIMINATED'
      case PHASE.POINTING:
        return isMyTurnToPoint
          ? 'Pick a Player!'
          : `Waiting for ${pointerName} to point...`
      case PHASE.BUZZER:
        return 'BUZZ TO ANSWER!'
      case PHASE.DECISION:
        return 'Make a Decision!'
      case PHASE.SPECTATING_DECISION:
        return `${deciderName} is deciding...`
      case PHASE.ANSWERING:
        return 'YOUR TURN!'
      case PHASE.WAITING:
        return `${activePlayerName} is answering...`
      case PHASE.WAITING_FOR_PRESENTER:
        return `Waiting for Presenter...`
      case PHASE.FINISHED:
        return 'Game Over'
      default:
        return ''
    }
  })

  return {
    get myContestant() {
      return myContestant
    },
    get isReady() {
      return isReady
    },
    get isActivePlayer() {
      return isActivePlayer
    },
    get isEliminated() {
      return isEliminated
    },
    get isRound1() {
      return isRound1
    },
    get isRound2() {
      return isRound2
    },
    get isRound3() {
      return isRound3
    },
    get isPointingPhase() {
      return isPointingPhase
    },
    get isMyTurnToPoint() {
      return isMyTurnToPoint
    },
    get currentPhase() {
      return currentPhase
    },
    get statusMessage() {
      return statusMessage
    },
    get activePlayerName() {
      return activePlayerName
    },
    get pointerName() {
      return pointerName
    },
    get deciderName() {
      return deciderName
    }
  }
}
