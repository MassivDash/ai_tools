/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import AgentConfig from './AgentConfig.svelte'
import { axiosBackendInstance } from '../../../axiosInstance/axiosBackendInstance.ts'
import type { Component } from 'svelte'

// Mock axiosBackendInstance
vi.mock('../../../axiosInstance/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    get: vi.fn(),
    post: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(console, 'log').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

const defaultProps = {
  isOpen: true,
  onClose: vi.fn(),
  onSave: vi.fn()
}

test('loads initial config state', async () => {
  mockedAxios.get.mockImplementation((url) => {
    if (url === 'agent/config')
      return Promise.resolve({
        data: { enabled_tools: ['calculator'] }
      })
    if (url === 'agent/tools') return Promise.resolve({ data: [] }) // Return empty array for tools
    return Promise.resolve({ data: {} })
  })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => {
    expect(screen.getByText('Agent Configuration')).toBeTruthy()
  })

  expect(mockedAxios.get).toHaveBeenCalledWith('agent/config')
})

test('calls onSave when save is successful', async () => {
  mockedAxios.get.mockImplementation((url) => {
    if (url === 'agent/config')
      return Promise.resolve({ data: { enabled_tools: [] } })
    if (url === 'agent/tools') return Promise.resolve({ data: [] })
    return Promise.resolve({ data: {} })
  })
  mockedAxios.post.mockResolvedValue({ data: { success: true } })

  const onSave = vi.fn()
  render(AgentConfig as Component, { props: { ...defaultProps, onSave } })

  const saveBtn = screen.getByText('Save')
  await fireEvent.click(saveBtn)

  await waitFor(() => {
    expect(onSave).toHaveBeenCalled()
  })
})
