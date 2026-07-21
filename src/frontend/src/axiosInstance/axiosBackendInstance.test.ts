import {
  axiosBackendInstance,
  getBackendUrl
} from '../axiosInstance/axiosBackendInstance'
import { describe, it, expect, vi, afterEach } from 'vitest'

describe('axiosBackendInstance', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('falls back to http://localhost:8080/api when PUBLIC_API_URL is not set', () => {
    vi.stubEnv('PUBLIC_API_URL', '')
    expect(getBackendUrl()).toBe('http://localhost:8080/api')
  })

  it('uses PUBLIC_API_URL when it is set', () => {
    vi.stubEnv('PUBLIC_API_URL', 'https://example.com/api')
    expect(getBackendUrl()).toBe('https://example.com/api')
  })

  it('is created with a non-empty baseURL', () => {
    expect(axiosBackendInstance.defaults.baseURL).toBeTruthy()
  })
})
