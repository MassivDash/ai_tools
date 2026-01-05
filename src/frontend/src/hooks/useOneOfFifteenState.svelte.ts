/* eslint-disable no-undef */
import { useWebSocket, type WebSocketOptions } from './useWebSocket'

export type UserRole = 'presenter' | 'contestant' | 'viewer'

export interface Contestant {
  name: string
  age: string
  score: number
  id: string
  session_id: string
  online: boolean
  ready: boolean
  lives: number
  round1_misses: number
  round1_questions: number
  eliminated: boolean
}

export type Round = 'lobby' | 'round1' | 'round2' | 'round3' | 'finished'

export interface Question {
  text: string
  correct_answer: string
  options?: string[]
}

export interface GameStateSnapshot {
  has_presenter: boolean
  presenter_online: boolean
  contestants: Contestant[]
  round: Round
  active_player_id?: string
  current_question?: Question
  timer_start?: number
  decision_pending?: boolean
}

export interface OneOfFifteenState {
  role: UserRole | null
  gameState: GameStateSnapshot
  error: string
}

import { useGameSession } from './useGameSession'

export function useOneOfFifteenState() {
  const { sessionId, clearSession } = useGameSession('1-of-15')

  let state = $state<OneOfFifteenState>({
    role: null,
    gameState: {
      has_presenter: false,
      presenter_online: false,
      contestants: [],
      round: 'lobby'
    },
    error: ''
  })
  let connected = $state(false)

  const getWebSocketUrl = (): string => {
    let baseUrl = import.meta.env.PUBLIC_API_URL || window.location.origin
    baseUrl = baseUrl.replace(/\/api\/?$/, '')
    baseUrl = baseUrl.replace(/\/$/, '')
    const wsProtocol = baseUrl.startsWith('https') ? 'wss' : 'ws'
    const wsBase = baseUrl.replace(/^https?:\/\//, '')
    return `${wsProtocol}://${wsBase}/api/games/1-of-15/ws`
  }

  const options: WebSocketOptions = {
    url: getWebSocketUrl(),
    onMessage: (event) => {
      try {
        const msg = JSON.parse(event.data)
        if (msg.type === 'welcome') {
          state.role = msg.role
          state.error = ''
        } else if (msg.type === 'state_update') {
          state.gameState = {
            has_presenter: msg.has_presenter,
            presenter_online: msg.presenter_online,
            contestants: msg.contestants,
            round: msg.round,
            active_player_id: msg.active_player_id,
            current_question: msg.current_question,
            timer_start: msg.timer_start, // Added
            decision_pending: msg.decision_pending
          }
        } else if (msg.type === 'error') {
          state.error = msg.message
        }
      } catch (err) {
        console.error('Failed to parse game message:', err)
      }
    },
    onOpen: () => {
      connected = true
      // Identify immediately
      if (sessionId) {
        sendMessage({ type: 'identify', session_id: sessionId })
      }
      startPolling()
    },
    onClose: () => {
      connected = false
      stopPolling()
    },
    onError: () => {
      connected = false
    }
  }

  const { connect, disconnect, send: wsSend } = useWebSocket(options)

  const sendMessage = (msg: any) => {
    // If identifying, we might not be fully "ready" logic-wise but socket is open.
    if (wsSend(JSON.stringify(msg))) {
      // success
    }
  }

  let pollInterval: any = null

  const startPolling = () => {
    if (pollInterval) return
    pollInterval = setInterval(() => {
      sendMessage({ type: 'get_state' })
    }, 2000)
  }

  const stopPolling = () => {
    if (pollInterval) {
      clearInterval(pollInterval)
      pollInterval = null
    }
  }

  const joinPresenter = () => {
    sendMessage({ type: 'join_presenter' })
    setTimeout(() => sendMessage({ type: 'get_state' }), 100)
  }

  const joinContestant = (name: string, age: string) => {
    sendMessage({ type: 'join_contestant', name, age })
    setTimeout(() => sendMessage({ type: 'get_state' }), 100)
  }

  const logout = () => {
    clearSession()
    disconnectWrapper()
    state.role = null
    window.location.reload()
  }

  const startGame = () => {
    sendMessage({ type: 'start_game' })
    setTimeout(() => sendMessage({ type: 'get_state' }), 100)
  }

  const resetGame = () => {
    sendMessage({ type: 'reset_game' })
    setTimeout(() => sendMessage({ type: 'get_state' }), 100)
  }

  const toggleReady = () => {
    sendMessage({ type: 'toggle_ready' })
    setTimeout(() => sendMessage({ type: 'get_state' }), 100)
  }

  const submitAnswer = (answer: string) => {
    sendMessage({ type: 'submit_answer', answer })
    setTimeout(() => sendMessage({ type: 'get_state' }), 100)
  }

  const pointToPlayer = (targetId: string) => {
    sendMessage({ type: 'point_to_player', target_id: targetId })
    setTimeout(() => sendMessage({ type: 'get_state' }), 100)
  }

  const buzzIn = () => {
    sendMessage({ type: 'buzz_in' })
  }

  const makeDecision = (choice: 'self' | 'point', targetId?: string) => {
    sendMessage({ type: 'make_decision', choice, target_id: targetId })
  }

  const disconnectWrapper = () => {
    stopPolling()
    disconnect()
    connected = false
  }

  return {
    state,
    get isConnected() {
      return connected
    },
    connect,
    disconnect: disconnectWrapper,
    joinPresenter,
    joinContestant,
    logout,
    startGame,
    resetGame,
    toggleReady,
    submitAnswer,
    pointToPlayer,
    buzzIn,
    makeDecision,
    sessionId
  }
}
