import { writable } from 'svelte/store'

interface ChatLayout {
  height: number // pixels
  width?: number // pixels, optional, if undefined defaults to 100% or max-width
}

const DEFAULT_HEIGHT = 600
const DEFAULT_WIDTH = 1000

function getStoredLayout(): ChatLayout {
  if (typeof window === 'undefined') return { height: DEFAULT_HEIGHT, width: DEFAULT_WIDTH }
  const stored = localStorage.getItem('chatLayout')
  if (stored) {
    try {
      const parsed = JSON.parse(stored)
      return { 
        height: parsed.height || DEFAULT_HEIGHT,
        width: parsed.width || DEFAULT_WIDTH 
      }
    } catch {
      return { height: DEFAULT_HEIGHT, width: DEFAULT_WIDTH }
    }
  }
  if (typeof window !== 'undefined') {
     return { 
       height: Math.floor(window.innerHeight * 0.6),
       width: Math.min(1024, window.innerWidth - 40)
     }
  }
  return { height: DEFAULT_HEIGHT, width: DEFAULT_WIDTH }
}

function createChatLayoutStore() {
  const { subscribe, update } = writable<ChatLayout>(getStoredLayout())

  return {
    subscribe,
    setHeight: (height: number) => {
        update(n => {
            const newState = { ...n, height }
            if (typeof localStorage !== 'undefined') {
                localStorage.setItem('chatLayout', JSON.stringify(newState))
            }
            return newState
        })
    },
    setWidth: (width: number) => {
        update(n => {
            const newState = { ...n, width }
            if (typeof localStorage !== 'undefined') {
                localStorage.setItem('chatLayout', JSON.stringify(newState))
            }
            return newState
        })
    },
    setDimensions: (height: number, width: number) => {
        update(n => {
            const newState = { ...n, height, width }
            if (typeof localStorage !== 'undefined') {
                localStorage.setItem('chatLayout', JSON.stringify(newState))
            }
            return newState
        })
    } 
  }
}

export const chatLayout = createChatLayoutStore()
