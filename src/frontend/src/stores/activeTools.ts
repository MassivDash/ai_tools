import { writable } from 'svelte/store'

// Store for enabled tools from config (persistent) - these show as badges in header
export const enabledTools = writable<Set<string>>(new Set())

// Alias for backwards compatibility - badges should show enabled tools from config
export const activeTools = enabledTools
