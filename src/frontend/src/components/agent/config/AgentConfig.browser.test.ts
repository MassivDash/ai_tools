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

const configGet =
  (config: Record<string, unknown>, tools: unknown[] = []) =>
  (url: string) => {
    if (url === 'agent/config') return Promise.resolve({ data: config })
    if (url === 'agent/tools') return Promise.resolve({ data: tools })
    return Promise.resolve({ data: {} })
  }

const mockTools = [
  {
    name: 'Calculator',
    tool_type: 'calculator',
    description: 'Perform math',
    category: 'utility',
    icon: 'calculator'
  },
  {
    name: 'Web Search',
    tool_type: 'web_search',
    description: 'Search the web',
    category: 'search',
    icon: 'magnify'
  }
]

test('does not load the config while the panel is closed', async () => {
  mockedAxios.get.mockImplementation(configGet({ enabled_tools: [] }))

  render(AgentConfig as Component, {
    props: { ...defaultProps, isOpen: false }
  })

  // ToolsConfigSection still loads the tool list on mount, but the panel
  // itself must not fetch the config until it is opened.
  await waitFor(() => expect(mockedAxios.get).toHaveBeenCalled())
  expect(mockedAxios.get).not.toHaveBeenCalledWith('agent/config')
})

test('tolerates a config response without enabled_tools and disables Clear All', async () => {
  mockedAxios.get.mockImplementation(configGet({}))

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() =>
    expect(mockedAxios.get).toHaveBeenCalledWith('agent/config')
  )
  expect(screen.getByRole('button', { name: 'Clear All' })).toBeDisabled()
})

test('logs a failure to load the config', async () => {
  const failure = new Error('config unavailable')
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/config') return Promise.reject(failure)
    return Promise.resolve({ data: [] })
  })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to load agent config:',
      failure
    )
  })
})

test('restores the debug logging flag from the backend', async () => {
  mockedAxios.get.mockImplementation(
    configGet({ enabled_tools: [], debug_logging: true })
  )

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => {
    expect(
      screen.getByLabelText('Debug Conversation Logging') as HTMLInputElement
    ).toBeChecked()
  })
})

test('toggling a tool on and off is reflected in the saved payload', async () => {
  // A save is followed by a reload, so the fake backend has to remember what
  // was posted for the second half of this test to be meaningful.
  let stored: string[] = ['calculator']
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/config')
      return Promise.resolve({ data: { enabled_tools: [...stored] } })
    if (url === 'agent/tools') return Promise.resolve({ data: mockTools })
    return Promise.resolve({ data: {} })
  })
  mockedAxios.post.mockImplementation((_url: string, payload: any) => {
    stored = payload.enabled_tools
    return Promise.resolve({ data: { success: true } })
  })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => screen.getByLabelText('Web Search'))
  await waitFor(() =>
    expect(
      screen.getByLabelText('Calculator') as HTMLInputElement
    ).toBeChecked()
  )

  // Enable a second tool
  await fireEvent.click(screen.getByLabelText('Web Search'))
  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      enabled_tools: ['calculator', 'web_search'],
      debug_logging: false
    })
  })

  // ...and disable the original one
  mockedAxios.post.mockClear()
  await fireEvent.click(screen.getByLabelText('Calculator'))
  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      enabled_tools: ['web_search'],
      debug_logging: false
    })
  })
})

test('saving persists the debug logging checkbox', async () => {
  mockedAxios.get.mockImplementation(configGet({ enabled_tools: [] }))
  mockedAxios.post.mockResolvedValue({ data: { success: true } })

  render(AgentConfig as Component, { props: defaultProps })

  await waitFor(() => screen.getByText('Agent Configuration'))
  await fireEvent.click(screen.getByLabelText('Debug Conversation Logging'))
  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/config', {
      enabled_tools: [],
      debug_logging: true
    })
  })
})

test('an unsuccessful save shows the returned message and keeps the panel open', async () => {
  mockedAxios.get.mockImplementation(configGet({ enabled_tools: [] }))
  mockedAxios.post.mockResolvedValue({
    data: { success: false, message: 'tool registry is locked' }
  })

  const onClose = vi.fn()
  const onSave = vi.fn()
  render(AgentConfig as Component, {
    props: { ...defaultProps, onClose, onSave }
  })

  await waitFor(() => screen.getByText('Agent Configuration'))
  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() =>
    expect(screen.getByText('tool registry is locked')).toBeTruthy()
  )
  expect(onClose).not.toHaveBeenCalled()
  expect(onSave).not.toHaveBeenCalled()
})

test.each([
  [
    'the backend error field',
    { response: { data: { error: 'bad tool type' } } },
    'bad tool type'
  ],
  [
    'the backend message field',
    { response: { data: { message: 'validation failed' } } },
    'validation failed'
  ],
  ['the thrown message', new Error('Network Error'), 'Network Error'],
  ['a generic fallback', {}, 'Failed to save agent config']
])('a rejected save surfaces %s', async (_label, rejection, expected) => {
  mockedAxios.get.mockImplementation(configGet({ enabled_tools: [] }))
  mockedAxios.post.mockRejectedValue(rejection)

  const onClose = vi.fn()
  render(AgentConfig as Component, { props: { ...defaultProps, onClose } })

  await waitFor(() => screen.getByText('Agent Configuration'))
  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() => expect(screen.getByText(expected)).toBeTruthy())
  expect(onClose).not.toHaveBeenCalled()
})

test('a successful save closes the panel and reloads the config', async () => {
  mockedAxios.get.mockImplementation(configGet({ enabled_tools: [] }))
  mockedAxios.post.mockResolvedValue({ data: { success: true } })

  const onClose = vi.fn()
  render(AgentConfig as Component, { props: { ...defaultProps, onClose } })

  await waitFor(() => screen.getByText('Agent Configuration'))
  const configLoadsBefore = mockedAxios.get.mock.calls.filter(
    ([url]: [string]) => url === 'agent/config'
  ).length

  await fireEvent.click(screen.getByText('Save'))

  await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1))
  const configLoadsAfter = mockedAxios.get.mock.calls.filter(
    ([url]: [string]) => url === 'agent/config'
  ).length
  expect(configLoadsAfter).toBe(configLoadsBefore + 1)
})

test('the close button and Cancel both call onClose', async () => {
  mockedAxios.get.mockImplementation(configGet({ enabled_tools: [] }))
  const onClose = vi.fn()

  render(AgentConfig as Component, { props: { ...defaultProps, onClose } })

  await waitFor(() => screen.getByText('Agent Configuration'))
  await fireEvent.click(screen.getByLabelText('Close'))
  await fireEvent.click(screen.getByText('Cancel'))

  expect(onClose).toHaveBeenCalledTimes(2)
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
