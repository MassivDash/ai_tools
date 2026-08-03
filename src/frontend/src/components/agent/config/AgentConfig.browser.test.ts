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
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
  put: ReturnType<typeof vi.fn>
  delete: ReturnType<typeof vi.fn>
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

test('switches to the Groups tab and applying a group merges its tools into the saved config', async () => {
  mockedAxios.get.mockImplementation((url) => {
    if (url === 'agent/config')
      return Promise.resolve({ data: { enabled_tools: ['calculator'] } })
    if (url === 'agent/tools') return Promise.resolve({ data: [] })
    if (url === 'agent/tool-groups')
      return Promise.resolve({
        data: {
          groups: [
            {
              id: 1,
              name: 'post writer',
              tool_types: ['bluesky_post', 'facebook_post'],
              created_at: 0,
              updated_at: 0
            }
          ]
        }
      })
    return Promise.resolve({ data: {} })
  })
  mockedAxios.post.mockResolvedValue({ data: { success: true } })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Agent Configuration'))

  await fireEvent.click(screen.getByRole('button', { name: 'Groups' }))
  await waitFor(() => screen.getByText('post writer'))

  await fireEvent.click(screen.getByRole('button', { name: /apply/i }))
  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      enabled_tools: ['calculator', 'bluesky_post', 'facebook_post'],
      debug_logging: false
    })
  })
})

test('applying a group persists immediately, without clicking the footer Save button', async () => {
  mockedAxios.get.mockImplementation((url) => {
    if (url === 'agent/config')
      return Promise.resolve({ data: { enabled_tools: ['calculator'] } })
    if (url === 'agent/tools') return Promise.resolve({ data: [] })
    if (url === 'agent/tool-groups')
      return Promise.resolve({
        data: {
          groups: [
            {
              id: 1,
              name: 'post writer',
              tool_types: ['bluesky_post', 'facebook_post'],
              created_at: 0,
              updated_at: 0
            }
          ]
        }
      })
    return Promise.resolve({ data: {} })
  })
  mockedAxios.post.mockResolvedValue({ data: { success: true } })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Agent Configuration'))
  await fireEvent.click(screen.getByRole('button', { name: 'Groups' }))
  await waitFor(() => screen.getByText('post writer'))

  await fireEvent.click(screen.getByRole('button', { name: /^apply$/i }))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      enabled_tools: ['calculator', 'bluesky_post', 'facebook_post'],
      debug_logging: false
    })
  })
})

test('removing a fully-applied group persists immediately, without clicking the footer Save button', async () => {
  mockedAxios.get.mockImplementation((url) => {
    if (url === 'agent/config')
      return Promise.resolve({
        data: { enabled_tools: ['calculator', 'bluesky_post', 'facebook_post'] }
      })
    if (url === 'agent/tools') return Promise.resolve({ data: [] })
    if (url === 'agent/tool-groups')
      return Promise.resolve({
        data: {
          groups: [
            {
              id: 1,
              name: 'post writer',
              tool_types: ['bluesky_post', 'facebook_post'],
              created_at: 0,
              updated_at: 0
            }
          ]
        }
      })
    return Promise.resolve({ data: {} })
  })
  mockedAxios.post.mockResolvedValue({ data: { success: true } })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Agent Configuration'))
  await fireEvent.click(screen.getByRole('button', { name: 'Groups' }))
  await waitFor(() => screen.getByText('post writer'))

  await fireEvent.click(screen.getByRole('button', { name: /^remove$/i }))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      enabled_tools: ['calculator'],
      debug_logging: false
    })
  })
  // The panel should stay open — removing a group is not the same as closing/saving the whole panel.
  expect(screen.getByText('Agent Configuration')).toBeTruthy()
})

test('Clear All persists immediately, without clicking the footer Save button', async () => {
  mockedAxios.get.mockImplementation((url) => {
    if (url === 'agent/config')
      return Promise.resolve({ data: { enabled_tools: ['calculator'] } })
    if (url === 'agent/tools') return Promise.resolve({ data: [] })
    return Promise.resolve({ data: {} })
  })
  mockedAxios.post.mockResolvedValue({ data: { success: true } })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Agent Configuration'))
  await fireEvent.click(screen.getByRole('button', { name: 'Clear All' }))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      enabled_tools: [],
      debug_logging: false
    })
  })
})

test('a fully-applied group shows Remove, and removing it un-checks its tools', async () => {
  mockedAxios.get.mockImplementation((url) => {
    if (url === 'agent/config')
      return Promise.resolve({
        data: { enabled_tools: ['calculator', 'bluesky_post', 'facebook_post'] }
      })
    if (url === 'agent/tools') return Promise.resolve({ data: [] })
    if (url === 'agent/tool-groups')
      return Promise.resolve({
        data: {
          groups: [
            {
              id: 1,
              name: 'post writer',
              tool_types: ['bluesky_post', 'facebook_post'],
              created_at: 0,
              updated_at: 0
            }
          ]
        }
      })
    return Promise.resolve({ data: {} })
  })
  mockedAxios.post.mockResolvedValue({ data: { success: true } })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Agent Configuration'))
  await fireEvent.click(screen.getByRole('button', { name: 'Groups' }))
  await waitFor(() => screen.getByText('post writer'))

  await fireEvent.click(screen.getByRole('button', { name: /^remove$/i }))
  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      enabled_tools: ['calculator'],
      debug_logging: false
    })
  })
})

test('Clear All empties the enabled tools', async () => {
  mockedAxios.get.mockImplementation((url) => {
    if (url === 'agent/config')
      return Promise.resolve({ data: { enabled_tools: ['calculator'] } })
    if (url === 'agent/tools') return Promise.resolve({ data: [] })
    return Promise.resolve({ data: {} })
  })
  mockedAxios.post.mockResolvedValue({ data: { success: true } })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Agent Configuration'))
  await fireEvent.click(screen.getByRole('button', { name: 'Clear All' }))
  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      enabled_tools: [],
      debug_logging: false
    })
  })
})
