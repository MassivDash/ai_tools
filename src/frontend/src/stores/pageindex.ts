import { writable } from 'svelte/store'
import type { PageIndexDocument } from '../types/pageindex'

export const documents = writable<PageIndexDocument[]>([])
export const selectedDocument = writable<PageIndexDocument | null>(null)
