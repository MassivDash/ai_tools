import { writable } from 'svelte/store'
import type { ChromaDBCollection } from '../types/chromadb'

export const collections = writable<ChromaDBCollection[]>([])
export const selectedCollection = writable<ChromaDBCollection | null>(null)

