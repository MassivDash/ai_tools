import { axiosBackendInstance } from '../axiosInstance/axiosBackendInstance'
import {
  SDConfigSchema,
  type SDConfig,
  type SDModelSet
} from '../validation/stableDiffusion'

export const stableDiffusionApi = {
  // Config Operations
  getSDConfig: async (): Promise<SDConfig | null> => {
    try {
      const resp = await axiosBackendInstance.get('sd-server/config')
      return resp.data
    } catch (e) {
      console.error('Failed to load SD config', e)
      throw e
    }
  },

  saveSDConfig: async (config: SDConfig): Promise<void> => {
    const parseResult = SDConfigSchema.safeParse(config)
    if (!parseResult.success) {
      const errorMsg = parseResult.error.issues
        .map((i) => `${i.path.join('.')}: ${i.message}`)
        .join(', ')
      throw new Error(errorMsg)
    }

    const sanitizedConfig = { ...config }
    for (const key in sanitizedConfig) {
      const k = key as keyof SDConfig
      if (
        typeof sanitizedConfig[k] === 'string' &&
        (sanitizedConfig[k] as string).trim() === ''
      ) {
         // @ts-ignore
         sanitizedConfig[k] = null
      }
    }

    const response = await axiosBackendInstance.post(
      'sd-server/config',
      sanitizedConfig
    )

    if (!response.data.success) {
      throw new Error(response.data.message || 'Failed to save config')
    }
  },

  // Model Set Operations
  getModelSets: async (): Promise<SDModelSet[]> => {
    try {
      const resp = await axiosBackendInstance.get('sd-server/model-sets')
      return resp.data || []
    } catch (e) {
      console.error('Failed to fetch model sets', e)
      throw e
    }
  },

  createModelSet: async (set: Omit<SDModelSet, 'id'>): Promise<void> => {
    try {
      await axiosBackendInstance.post('sd-server/model-sets', set)
    } catch (e: any) {
      console.error('Failed to save set', e)
      throw new Error(e.response?.data || 'Failed to save set')
    }
  },

  updateModelSet: async (id: number, set: Partial<SDModelSet>): Promise<void> => {
    try {
      await axiosBackendInstance.put(`sd-server/model-sets/${id}`, set)
    } catch (e: any) {
      console.error('Failed to update set', e)
      throw new Error(e.response?.data || 'Failed to update set')
    }
  },

  deleteModelSet: async (id: number): Promise<void> => {
    try {
      await axiosBackendInstance.delete(`sd-server/model-sets/${id}`)
    } catch (e: any) {
      console.error('Failed to delete set', e)
      throw new Error(e.response?.data || 'Failed to delete set')
    }
  }
}
