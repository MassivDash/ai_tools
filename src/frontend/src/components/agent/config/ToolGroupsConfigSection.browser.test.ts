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
