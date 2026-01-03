/* eslint-disable no-undef */
import { useWebSocket, type WebSocketOptions } from './useWebSocket'

export type UserRole = 'presenter' | 'contestant' | 'viewer'

export interface Contestant {
  name: string
  score: number
  id: string
  session_id: string
  online: boolean
  ready: boolean
}

export type GameStatus = 'lobby' | 'playing' | 'finished'

export interface GameStateSnapshot {
  has_presenter: boolean
  presenter_online: boolean
  contestants: Contestant[]
  status: GameStatus
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
      status: 'lobby'
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
            presenter_online: msg.presenter_online, // Added
            contestants: msg.contestants,
            status: msg.status
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

  const joinContestant = (name: string) => {
    sendMessage({ type: 'join_contestant', name })
    setTimeout(() => sendMessage({ type: 'get_state' }), 100)
  }

  const logout = () => {
    clearSession()
    disconnectWrapper()
    state.role = null
    // Use window.location.reload() to fully reset state/session for now?
    // Or just clear local state.
    // If we clear session, we should probably generate a new one if they want to rejoin as different person without reload.
    // But useGameSession hook runs once.
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
    sessionId
  }
}
// Need to remove 'derived' usage if not imported, accessing $state directly in getter is reactive in Svelte 5.
