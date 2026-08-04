/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import ToolGroupsConfigSection from './ToolGroupsConfigSection.svelte'
import { axiosBackendInstance } from '../../../axiosInstance/axiosBackendInstance.ts'
import type { Component } from 'svelte'

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
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
})

const mockTools = [
  {
    id: 'bluesky_post',
    name: 'Bluesky Post',
    tool_type: 'bluesky_post',
    description: 'Post to Bluesky',
    category: 'social',
    icon: 'send'
  },
  {
    id: 'facebook_post',
    name: 'Facebook Post',
    tool_type: 'facebook_post',
    description: 'Post to Facebook',
    category: 'social',
    icon: 'send'
  }
]

const mockGroups = [
  {
    id: 1,
    name: 'post writer',
    tool_types: ['bluesky_post', 'facebook_post'],
    created_at: 0,
    updated_at: 0
  }
]

const mockGetImplementation =
  (groups = mockGroups) =>
  (url: string) => {
    if (url === 'agent/tool-groups') {
      return Promise.resolve({ data: { groups } })
    }
    if (url === 'agent/tools') {
      return Promise.resolve({ data: mockTools })
    }
    return Promise.reject(new Error(`Unexpected GET ${url}`))
  }

const baseProps = () => ({
  enabledTools: [] as string[],
  onApply: vi.fn(),
  onRemove: vi.fn()
})

test('renders empty state when there are no groups', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation([]))
  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => {
    expect(screen.getByText(/no tool groups/i)).toBeTruthy()
  })
})

test('renders saved groups from the API', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => {
    expect(screen.getByText('post writer')).toBeTruthy()
  })
})

test('clicking Apply calls onApply with the group tool_types and shows a confirmation', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  const onApply = vi.fn()
  render(ToolGroupsConfigSection as Component, {
    props: { ...baseProps(), onApply }
  })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getByRole('button', { name: /^apply$/i }))

  expect(onApply).toHaveBeenCalledWith(['bluesky_post', 'facebook_post'])
  await waitFor(() => {
    expect(screen.getByText(/applied "post writer"/i)).toBeTruthy()
  })
})

test('flips from Apply to Remove when enabledTools updates on an already-rendered instance', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  const { rerender } = render(ToolGroupsConfigSection as Component, {
    props: baseProps()
  })

  await waitFor(() => screen.getByText('post writer'))
  expect(screen.getByRole('button', { name: /^apply$/i })).toBeTruthy()

  await rerender({
    ...baseProps(),
    enabledTools: ['bluesky_post', 'facebook_post']
  })

  await waitFor(() => {
    expect(screen.getByRole('button', { name: /^remove$/i })).toBeTruthy()
  })
  expect(
    screen.queryByRole('button', { name: /^apply$/i })
  ).not.toBeInTheDocument()
})

test('shows Remove instead of Apply once every group tool is already enabled', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  render(ToolGroupsConfigSection as Component, {
    props: {
      ...baseProps(),
      enabledTools: ['bluesky_post', 'facebook_post', 'other_tool']
    }
  })

  await waitFor(() => screen.getByText('post writer'))
  expect(screen.getByRole('button', { name: /^remove$/i })).toBeTruthy()
  expect(
    screen.queryByRole('button', { name: /^apply$/i })
  ).not.toBeInTheDocument()
})

test('still shows Apply when only some of the group tools are enabled, and applying activates the rest', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  const onApply = vi.fn()
  render(ToolGroupsConfigSection as Component, {
    props: { ...baseProps(), enabledTools: ['bluesky_post'], onApply }
  })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getByRole('button', { name: /^apply$/i }))

  expect(onApply).toHaveBeenCalledWith(['bluesky_post', 'facebook_post'])
})

test('clicking Remove calls onRemove with the group tool_types and shows a confirmation', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  const onRemove = vi.fn()
  render(ToolGroupsConfigSection as Component, {
    props: {
      ...baseProps(),
      enabledTools: ['bluesky_post', 'facebook_post'],
      onRemove
    }
  })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getByRole('button', { name: /^remove$/i }))

  expect(onRemove).toHaveBeenCalledWith(['bluesky_post', 'facebook_post'])
  await waitFor(() => {
    expect(screen.getByText(/removed "post writer"/i)).toBeTruthy()
  })
})

test('creating a new group posts the name and selected tools', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation([]))
  mockedAxios.post.mockResolvedValue({
    data: {
      group: {
        id: 2,
        name: 'new group',
        tool_types: ['bluesky_post'],
        created_at: 0,
        updated_at: 0
      }
    }
  })

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText(/new group/i))
  await fireEvent.click(screen.getByRole('button', { name: /new group/i }))

  await waitFor(() => screen.getByLabelText('Bluesky Post'))
  await fireEvent.input(screen.getByPlaceholderText('Group name'), {
    target: { value: 'new group' }
  })
  await fireEvent.click(screen.getByLabelText('Bluesky Post'))
  await fireEvent.click(screen.getByRole('button', { name: /^save$/i }))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledWith('agent/tool-groups', {
      name: 'new group',
      tool_types: ['bluesky_post']
    })
  })
})

test('shows the backend error message when loading groups fails', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/tool-groups') {
      return Promise.reject({ response: { data: { error: 'db offline' } } })
    }
    return Promise.resolve({ data: mockTools })
  })

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => {
    expect(screen.getByText('db offline')).toBeTruthy()
  })
  expect(screen.getByText(/no tool groups/i)).toBeTruthy()
})

test('falls back to the thrown error message when loading groups fails', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/tool-groups') {
      return Promise.reject(new Error('Network Error'))
    }
    return Promise.resolve({ data: mockTools })
  })

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => {
    expect(screen.getByText('Network Error')).toBeTruthy()
  })
})

test('falls back to a generic message when loading groups fails without detail', async () => {
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/tool-groups') return Promise.reject({})
    return Promise.resolve({ data: mockTools })
  })

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => {
    expect(screen.getByText('Failed to load tool groups')).toBeTruthy()
  })
})

test('a failed tools request is logged and leaves the picker empty', async () => {
  const failure = new Error('tools down')
  mockedAxios.get.mockImplementation((url: string) => {
    if (url === 'agent/tool-groups')
      return Promise.resolve({ data: { groups: [] } })
    return Promise.reject(failure)
  })

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText(/no tool groups/i))
  expect(console.error).toHaveBeenCalledWith('Failed to load tools:', failure)

  await fireEvent.click(screen.getByRole('button', { name: /new group/i }))
  expect(screen.queryByLabelText('Bluesky Post')).toBeNull()
})

test('unselecting a tool in the modal disables saving again', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation([]))

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByRole('button', { name: /new group/i }))
  await fireEvent.click(screen.getByRole('button', { name: /new group/i }))

  await waitFor(() => screen.getByLabelText('Bluesky Post'))
  await fireEvent.input(screen.getByPlaceholderText('Group name'), {
    target: { value: 'group' }
  })

  const checkbox = screen.getByLabelText('Bluesky Post') as HTMLInputElement
  await fireEvent.click(checkbox)
  expect(checkbox.checked).toBe(true)
  expect(screen.getByRole('button', { name: /^save$/i })).not.toBeDisabled()

  await fireEvent.click(checkbox)
  expect(checkbox.checked).toBe(false)
  expect(screen.getByRole('button', { name: /^save$/i })).toBeDisabled()
})

test('editing a group pre-fills the modal and puts the update, replacing only that group', async () => {
  const groups = [
    {
      id: 1,
      name: 'post writer',
      tool_types: ['bluesky_post'],
      created_at: 0,
      updated_at: 0
    },
    {
      id: 2,
      name: 'other group',
      tool_types: ['facebook_post'],
      created_at: 0,
      updated_at: 0
    }
  ]
  mockedAxios.get.mockImplementation(mockGetImplementation(groups))
  mockedAxios.put.mockResolvedValue({
    data: {
      group: {
        id: 1,
        name: 'renamed writer',
        tool_types: ['bluesky_post', 'facebook_post'],
        created_at: 0,
        updated_at: 1
      }
    }
  })

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getAllByRole('button', { name: /^edit$/i })[0])

  await waitFor(() => screen.getByText('Edit Tool Group'))
  const nameInput = screen.getByPlaceholderText(
    'Group name'
  ) as HTMLInputElement
  expect(nameInput.value).toBe('post writer')
  expect(
    (screen.getByLabelText('Bluesky Post') as HTMLInputElement).checked
  ).toBe(true)
  expect(
    (screen.getByLabelText('Facebook Post') as HTMLInputElement).checked
  ).toBe(false)

  await fireEvent.input(nameInput, { target: { value: 'renamed writer' } })
  await fireEvent.click(screen.getByLabelText('Facebook Post'))
  await fireEvent.click(screen.getByRole('button', { name: /^save$/i }))

  await waitFor(() => {
    expect(mockedAxios.put).toHaveBeenCalledWith('agent/tool-groups/1', {
      name: 'renamed writer',
      tool_types: ['bluesky_post', 'facebook_post']
    })
  })
  await waitFor(() => expect(screen.getByText('renamed writer')).toBeTruthy())
  expect(screen.getByText('other group')).toBeTruthy()
  expect(screen.queryByText('post writer')).toBeNull()
})

test('creating a group opens a modal titled for creation and appends the new group', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  mockedAxios.post.mockResolvedValue({
    data: {
      group: {
        id: 7,
        name: 'second group',
        tool_types: ['facebook_post'],
        created_at: 0,
        updated_at: 0
      }
    }
  })

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getByRole('button', { name: /new group/i }))

  await waitFor(() => screen.getByText('New Tool Group'))
  expect(
    (screen.getByPlaceholderText('Group name') as HTMLInputElement).value
  ).toBe('')

  await fireEvent.input(screen.getByPlaceholderText('Group name'), {
    target: { value: 'second group' }
  })
  await fireEvent.click(screen.getByLabelText('Facebook Post'))
  await fireEvent.click(screen.getByRole('button', { name: /^save$/i }))

  await waitFor(() => expect(screen.getByText('second group')).toBeTruthy())
  expect(screen.getByText('post writer')).toBeTruthy()
  expect(screen.queryByText('New Tool Group')).toBeNull()
})

test('a failed save keeps the modal open and shows the backend error', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation([]))
  mockedAxios.post.mockRejectedValue({
    response: { data: { error: 'name already taken' } }
  })

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByRole('button', { name: /new group/i }))
  await fireEvent.click(screen.getByRole('button', { name: /new group/i }))

  await waitFor(() => screen.getByLabelText('Bluesky Post'))
  await fireEvent.input(screen.getByPlaceholderText('Group name'), {
    target: { value: 'dupe' }
  })
  await fireEvent.click(screen.getByLabelText('Bluesky Post'))
  await fireEvent.click(screen.getByRole('button', { name: /^save$/i }))

  await waitFor(() =>
    expect(screen.getByText('name already taken')).toBeTruthy()
  )
  expect(screen.getByText('New Tool Group')).toBeTruthy()
})

test('a failed save without a backend error falls back to the thrown message', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation([]))
  mockedAxios.post.mockRejectedValue(new Error('Request failed'))

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByRole('button', { name: /new group/i }))
  await fireEvent.click(screen.getByRole('button', { name: /new group/i }))

  await waitFor(() => screen.getByLabelText('Bluesky Post'))
  await fireEvent.input(screen.getByPlaceholderText('Group name'), {
    target: { value: 'dupe' }
  })
  await fireEvent.click(screen.getByLabelText('Bluesky Post'))
  await fireEvent.click(screen.getByRole('button', { name: /^save$/i }))

  await waitFor(() => expect(screen.getByText('Request failed')).toBeTruthy())
})

test('a failed save without any detail falls back to a generic message', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation([]))
  mockedAxios.post.mockRejectedValue({})

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByRole('button', { name: /new group/i }))
  await fireEvent.click(screen.getByRole('button', { name: /new group/i }))

  await waitFor(() => screen.getByLabelText('Bluesky Post'))
  await fireEvent.input(screen.getByPlaceholderText('Group name'), {
    target: { value: 'dupe' }
  })
  await fireEvent.click(screen.getByLabelText('Bluesky Post'))
  await fireEvent.click(screen.getByRole('button', { name: /^save$/i }))

  await waitFor(() =>
    expect(screen.getByText('Failed to save tool group')).toBeTruthy()
  )
})

test('cancelling the modal discards the draft group', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation([]))

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByRole('button', { name: /new group/i }))
  await fireEvent.click(screen.getByRole('button', { name: /new group/i }))

  await waitFor(() => screen.getByLabelText('Bluesky Post'))
  await fireEvent.click(screen.getByRole('button', { name: /^cancel$/i }))

  await waitFor(() => expect(screen.queryByText('New Tool Group')).toBeNull())
  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('declining the delete confirmation leaves the group in place', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  vi.spyOn(window, 'confirm').mockReturnValue(false)

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getByRole('button', { name: /delete/i }))

  expect(window.confirm).toHaveBeenCalledWith('Delete group "post writer"?')
  expect(mockedAxios.delete).not.toHaveBeenCalled()
  expect(screen.getByText('post writer')).toBeTruthy()
})

test('a failed delete shows the backend error and keeps the group', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  mockedAxios.delete.mockRejectedValue({
    response: { data: { error: 'group is in use' } }
  })
  vi.spyOn(window, 'confirm').mockReturnValue(true)

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getByRole('button', { name: /delete/i }))

  await waitFor(() => expect(screen.getByText('group is in use')).toBeTruthy())
  expect(screen.getByText('post writer')).toBeTruthy()
})

test('a failed delete falls back to the thrown message', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  mockedAxios.delete.mockRejectedValue(new Error('Request failed'))
  vi.spyOn(window, 'confirm').mockReturnValue(true)

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getByRole('button', { name: /delete/i }))

  await waitFor(() => expect(screen.getByText('Request failed')).toBeTruthy())
})

test('a failed delete without any detail falls back to a generic message', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  mockedAxios.delete.mockRejectedValue({})
  vi.spyOn(window, 'confirm').mockReturnValue(true)

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getByRole('button', { name: /delete/i }))

  await waitFor(() =>
    expect(screen.getByText('Failed to delete tool group')).toBeTruthy()
  )
})

test('the applied toast is replaced when another group is applied', async () => {
  const groups = [
    {
      id: 1,
      name: 'post writer',
      tool_types: ['bluesky_post'],
      created_at: 0,
      updated_at: 0
    },
    {
      id: 2,
      name: 'other group',
      tool_types: ['facebook_post'],
      created_at: 0,
      updated_at: 0
    }
  ]
  mockedAxios.get.mockImplementation(mockGetImplementation(groups))

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText('post writer'))

  const applyButtons = screen.getAllByRole('button', { name: /^apply$/i })
  await fireEvent.click(applyButtons[0])
  expect(screen.getByRole('status').textContent).toContain(
    'Applied "post writer"'
  )

  await fireEvent.click(applyButtons[1])
  expect(screen.getAllByRole('status')).toHaveLength(1)
  expect(screen.getByRole('status').textContent).toContain(
    'Applied "other group"'
  )
})

test('deleting a group calls the delete endpoint after confirmation', async () => {
  mockedAxios.get.mockImplementation(mockGetImplementation())
  mockedAxios.delete.mockResolvedValue({ data: { success: true } })
  vi.spyOn(window, 'confirm').mockReturnValue(true)

  render(ToolGroupsConfigSection as Component, { props: baseProps() })

  await waitFor(() => screen.getByText('post writer'))
  await fireEvent.click(screen.getByRole('button', { name: /delete/i }))

  await waitFor(() => {
    expect(mockedAxios.delete).toHaveBeenCalledWith('agent/tool-groups/1')
  })
})
