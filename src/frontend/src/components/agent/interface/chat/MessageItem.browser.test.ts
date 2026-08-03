/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import MessageItem from './MessageItem.svelte'
import type { Component } from 'svelte'
import { icons } from '@iconify-json/mdi'

// chart.js needs a real 2d canvas context, which jsdom does not provide. Stub the
// constructor so <Chart> can mount and we can assert what MessageItem hands to it.
const chartMock = vi.hoisted(() => ({ configs: [] as any[] }))

vi.mock('chart.js/auto', () => ({
  default: class {
    options: any = {}
    data: any = { datasets: [] }
    constructor(_canvas: unknown, config: any) {
      chartMock.configs.push(config)
    }
    update() {}
    destroy() {}
  }
}))

// MessageItem -> utils/toolIcons -> axiosBackendInstance (tool metadata lookup)
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    get: vi.fn(() =>
      Promise.resolve({
        data: [{ name: 'search_web', tool_type: 'web', icon: 'web' }]
      })
    )
  }
}))

// The `d` attribute of an mdi icon, so we can assert *which* icon was rendered.
const iconPath = (name: string): string =>
  icons.icons[name].body.match(/d="([^"]+)"/)![1]

const renderedIconPaths = (): (string | null)[] =>
  Array.from(document.querySelectorAll('svg path')).map((p) =>
    p.getAttribute('d')
  )

const chartFence = (title: string): string =>
  '```json-chart\n' +
  JSON.stringify({
    type: 'bar',
    title,
    xAxis: { label: 'x', data: ['a', 'b'] },
    series: [{ name: 's1', data: [1, 2] }]
  }) +
  '\n```'

let clipboardWrite: ReturnType<typeof vi.fn>

beforeEach(() => {
  chartMock.configs.length = 0
  clipboardWrite = vi.fn().mockResolvedValue(undefined)
  Object.defineProperty(window.navigator, 'clipboard', {
    value: { writeText: clipboardWrite },
    configurable: true,
    writable: true
  })
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

test('renders user message', () => {
  const message = {
    id: '1',
    role: 'user',
    content: 'Hello AI',
    timestamp: Date.now()
  }

  render(MessageItem as Component, {
    props: { message }
  })

  expect(screen.getByText('Hello AI')).toBeTruthy()
  expect(screen.getByText('You')).toBeTruthy()
})

test('renders assistant message with markdown', () => {
  const message = {
    id: '2',
    role: 'assistant',
    content: '**Bold** response',
    timestamp: Date.now()
  }

  render(MessageItem as Component, {
    props: { message }
  })

  const container = document.querySelector('.message-content')
  expect(container).toBeTruthy()
  // Markdown is converted to HTML, not injected as literal text
  expect(container?.innerHTML).toContain('<strong>Bold</strong>')
  expect(container).toHaveClass('markdown')
})

test('renders tool calls', () => {
  const message = {
    id: '3',
    role: 'tool',
    content: '✅ tool_name completed',
    timestamp: Date.now(),
    toolName: 'search_web'
  }

  render(MessageItem as Component, {
    props: { message }
  })

  expect(screen.getByText(/tool_name/)).toBeTruthy()
  expect(screen.getByText(/completed/)).toBeTruthy()
})

test('renders image content array', () => {
  const message = {
    id: '4',
    role: 'user',
    content: [
      { type: 'text', text: 'Look at this:' },
      { type: 'image_url', image_url: { url: 'http://example.com/image.jpg' } }
    ],
    timestamp: Date.now()
  }

  render(MessageItem as Component, {
    props: { message }
  })

  expect(screen.getByText('Look at this:')).toBeTruthy()
  const img = document.querySelector('img')
  expect(img).toBeTruthy()
  expect(img?.src).toBe('http://example.com/image.jpg')
})

test('hides image attachments from chips', () => {
  const message = {
    id: '5',
    role: 'user',
    content: [{ type: 'image_url', image_url: { url: '...' } }],
    timestamp: Date.now(),
    attachments: [
      { name: 'photo.jpg', type: 'image', content: '...' },
      { name: 'doc.txt', type: 'text', content: '...' }
    ]
  }

  render(MessageItem as Component, {
    props: { message }
  })

  // Should show doc.txt chip
  expect(screen.getByText('doc.txt')).toBeTruthy()
  // Should NOT show photo.jpg chip (filtered out because type is image)
  expect(screen.queryByText('photo.jpg')).toBeNull()
})

test('renders AskHuman tool with options', () => {
  const message = {
    id: '6',
    role: 'tool',
    content: 'Waiting for your input...',
    timestamp: Date.now(),
    toolName: 'ask_human',
    toolCallId: 'call_123',
    toolArguments: JSON.stringify({
      question: 'Do you want to proceed?',
      options: ['Yes', 'No']
    })
  }

  render(MessageItem as Component, {
    props: { message }
  })

  expect(screen.getByText('Do you want to proceed?')).toBeTruthy()
  expect(screen.getByText('Yes')).toBeTruthy()
  expect(screen.getByText('No')).toBeTruthy()
  expect(screen.getByText('Other')).toBeTruthy()
})

test('renders AskHuman tool without options (fallback to Other)', () => {
  const message = {
    id: '7',
    role: 'tool',
    content: 'Waiting for your input...',
    timestamp: Date.now(),
    toolName: 'ask_human',
    toolCallId: 'call_124',
    toolArguments: JSON.stringify({
      question: 'Please specify your preference:'
      // Missing options
    })
  }

  render(MessageItem as Component, {
    props: { message }
  })

  expect(screen.getByText('Please specify your preference:')).toBeTruthy()
  // Even without options, 'Other' should be appended and visible
  expect(screen.getByText('Other')).toBeTruthy()
})

/* -------------------------------------------------------------------------- */
/* status messages                                                            */
/* -------------------------------------------------------------------------- */

const statusMessage = (
  statusType: string | undefined,
  content = 'Working'
) => ({
  id: 's',
  role: 'status',
  statusType,
  content,
  timestamp: Date.now()
})

test('renders a spinner and fixed label for the thinking status', () => {
  render(MessageItem as Component, {
    props: { message: statusMessage('thinking', 'ignored content') }
  })

  expect(document.querySelector('.status-message')).toBeTruthy()
  expect(document.querySelector('.spinning-cog')).toBeTruthy()
  expect(screen.getByText('Thinking...')).toBeTruthy()
  // The raw content is intentionally not shown for `thinking`
  expect(screen.queryByText('ignored content')).toBeNull()
  expect(renderedIconPaths()).toContain(iconPath('cog'))
})

test('renders the wrench icon and content for the calling_tool status', () => {
  render(MessageItem as Component, {
    props: { message: statusMessage('calling_tool', 'Calling search_web') }
  })

  expect(screen.getByText('Calling search_web')).toBeTruthy()
  expect(document.querySelector('.spinning-cog')).toBeNull()
  expect(renderedIconPaths()).toContain(iconPath('wrench'))
})

test('renders a spinner and content for the tool_executing status', () => {
  render(MessageItem as Component, {
    props: { message: statusMessage('tool_executing', 'Running search_web') }
  })

  expect(screen.getByText('Running search_web')).toBeTruthy()
  expect(document.querySelector('.spinning-cog')).toBeTruthy()
})

test('renders a success icon for the tool_complete status', () => {
  render(MessageItem as Component, {
    props: { message: statusMessage('tool_complete', 'search_web done') }
  })

  expect(screen.getByText('search_web done')).toBeTruthy()
  expect(document.querySelector('svg.success-icon')).toBeTruthy()
  expect(renderedIconPaths()).toContain(iconPath('check-circle'))
})

test('renders an error icon for the tool_error status', () => {
  render(MessageItem as Component, {
    props: { message: statusMessage('tool_error', 'search_web exploded') }
  })

  expect(screen.getByText('search_web exploded')).toBeTruthy()
  expect(document.querySelector('svg.error-icon')).toBeTruthy()
  expect(renderedIconPaths()).toContain(iconPath('close-circle'))
})

test('renders plain content for an unknown status type', () => {
  render(MessageItem as Component, {
    props: { message: statusMessage(undefined, 'Just some status') }
  })

  expect(screen.getByText('Just some status')).toBeTruthy()
  expect(document.querySelector('.spinning-cog')).toBeNull()
  expect(document.querySelector('svg.success-icon')).toBeNull()
  expect(document.querySelector('svg.error-icon')).toBeNull()
})

/* -------------------------------------------------------------------------- */
/* tool messages                                                              */
/* -------------------------------------------------------------------------- */

test('flags a successful tool message and strips the status emoji', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 't1',
        role: 'tool',
        content: '✅ search_web completed',
        timestamp: 1,
        toolName: 'search_web'
      }
    }
  })

  const indicator = document.querySelector('.tool-indicator')!
  expect(indicator).toHaveClass('success')
  expect(indicator).not.toHaveClass('error')
  expect(document.querySelector('.tool-text')?.textContent?.trim()).toBe(
    'search_web completed'
  )
  expect(document.querySelector('svg.status-icon.success-icon')).toBeTruthy()
})

test('flags a failed tool message and strips the status emoji', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 't2',
        role: 'tool',
        content: '❌ search_web failed',
        timestamp: 1,
        toolName: 'search_web'
      }
    }
  })

  const indicator = document.querySelector('.tool-indicator')!
  expect(indicator).toHaveClass('error')
  expect(indicator).not.toHaveClass('success')
  expect(document.querySelector('.tool-text')?.textContent?.trim()).toBe(
    'search_web failed'
  )
  expect(document.querySelector('svg.status-icon.error-icon')).toBeTruthy()
})

test('falls back to a generic label for non-string tool content', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 't3',
        role: 'tool',
        content: [{ type: 'text', text: 'nope' }],
        timestamp: 1,
        toolName: 'search_web'
      }
    }
  })

  expect(document.querySelector('.tool-text')?.textContent?.trim()).toBe(
    'Tool execution'
  )
  const indicator = document.querySelector('.tool-indicator')!
  expect(indicator).not.toHaveClass('success')
  expect(indicator).not.toHaveClass('error')
})

test('shows a waiting label for a pending ask_human tool message', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 't4',
        role: 'tool',
        content: 'ask_human',
        timestamp: 1,
        toolName: 'ask_human'
      }
    }
  })

  expect(document.querySelector('.tool-text')?.textContent?.trim()).toBe(
    'Waiting for your input...'
  )
  // No toolArguments -> no options block
  expect(document.querySelector('.ask-human-container')).toBeNull()
})

test('upgrades the tool icon from backend metadata', async () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 't5',
        role: 'tool',
        content: 'searching',
        timestamp: 1,
        toolName: 'search_web'
      }
    }
  })

  // Pattern-matched fallback lands on `magnify` first...
  expect(document.querySelector('svg.tool-icon path')?.getAttribute('d')).toBe(
    iconPath('magnify')
  )

  // ...then the metadata lookup swaps in the icon reported by the backend.
  await waitFor(() => {
    expect(
      document.querySelector('svg.tool-icon path')?.getAttribute('d')
    ).toBe(iconPath('web'))
  })
})

/* -------------------------------------------------------------------------- */
/* ask_human interaction                                                      */
/* -------------------------------------------------------------------------- */

const askHumanMessage = (args: unknown) => ({
  id: 'ah',
  role: 'tool',
  content: 'ask_human',
  timestamp: 1,
  toolName: 'ask_human',
  toolCallId: 'call_abc',
  toolArguments: typeof args === 'string' ? args : JSON.stringify(args)
})

test('submits the selected ask_human option', async () => {
  const onSubmitToolResult = vi.fn()

  render(MessageItem as Component, {
    props: {
      message: askHumanMessage({
        question: 'Proceed?',
        options: ['Yes', 'No']
      }),
      onSubmitToolResult
    }
  })

  const submit = screen.getByRole('button', { name: 'Submit' })
  expect(submit).toBeDisabled()

  await fireEvent.click(screen.getByDisplayValue('Yes'))
  expect(submit).not.toBeDisabled()

  await fireEvent.click(submit)
  expect(onSubmitToolResult).toHaveBeenCalledWith(
    'ask_human',
    'call_abc',
    'Yes'
  )
})

test('collects free text when the Other option is picked', async () => {
  const onSubmitToolResult = vi.fn()

  render(MessageItem as Component, {
    props: {
      message: askHumanMessage({ question: 'Proceed?', options: ['Yes'] }),
      onSubmitToolResult
    }
  })

  // The free-text field only appears once "Other" is selected
  expect(document.querySelector('.ask-human-other-input')).toBeNull()
  await fireEvent.click(screen.getByDisplayValue('Other'))

  const other = document.querySelector(
    '.ask-human-other-input'
  ) as HTMLInputElement
  expect(other).toBeTruthy()

  // Selected but empty -> still blocked
  const submit = screen.getByRole('button', { name: 'Submit' })
  expect(submit).toBeDisabled()

  await fireEvent.input(other, { target: { value: 'something else' } })
  expect(submit).not.toBeDisabled()

  await fireEvent.click(submit)
  expect(onSubmitToolResult).toHaveBeenCalledWith(
    'ask_human',
    'call_abc',
    'something else'
  )
})

test('submits the Other free text on Enter but ignores blank input', async () => {
  const onSubmitToolResult = vi.fn()

  render(MessageItem as Component, {
    props: {
      message: askHumanMessage({ question: 'Proceed?', options: ['Yes'] }),
      onSubmitToolResult
    }
  })

  await fireEvent.click(screen.getByDisplayValue('Other'))
  const other = document.querySelector(
    '.ask-human-other-input'
  ) as HTMLInputElement

  await fireEvent.input(other, { target: { value: '   ' } })
  await fireEvent.keyDown(other, { key: 'Enter' })
  expect(onSubmitToolResult).not.toHaveBeenCalled()

  await fireEvent.input(other, { target: { value: '  typed answer  ' } })
  await fireEvent.keyDown(other, { key: 'a' })
  expect(onSubmitToolResult).not.toHaveBeenCalled()

  await fireEvent.keyDown(other, { key: 'Enter' })
  expect(onSubmitToolResult).toHaveBeenCalledWith(
    'ask_human',
    'call_abc',
    'typed answer'
  )
})

test('clears the Other free text when another option is selected', async () => {
  render(MessageItem as Component, {
    props: {
      message: askHumanMessage({ question: 'Proceed?', options: ['Yes'] })
    }
  })

  await fireEvent.click(screen.getByDisplayValue('Other'))
  const other = document.querySelector(
    '.ask-human-other-input'
  ) as HTMLInputElement
  await fireEvent.input(other, { target: { value: 'draft text' } })

  await fireEvent.click(screen.getByDisplayValue('Yes'))
  expect(document.querySelector('.ask-human-other-input')).toBeNull()

  await fireEvent.click(screen.getByDisplayValue('Other'))
  expect(
    (document.querySelector('.ask-human-other-input') as HTMLInputElement).value
  ).toBe('')
})

test('does not duplicate an Other option supplied by the tool', () => {
  render(MessageItem as Component, {
    props: {
      message: askHumanMessage({
        question: 'Proceed?',
        options: ['Yes', 'Other']
      })
    }
  })

  const radios = document.querySelectorAll('input[type="radio"]')
  expect(radios).toHaveLength(2)
  expect(screen.getAllByDisplayValue('Other')).toHaveLength(1)
})

test('shows the raw arguments as the question when they are not valid JSON', () => {
  render(MessageItem as Component, {
    props: { message: askHumanMessage('{"question": "half written') }
  })

  expect(document.querySelector('.ask-human-question')?.textContent).toBe(
    '{"question": "half written'
  )
  // Only the synthetic "Other" option is offered
  expect(document.querySelectorAll('input[type="radio"]')).toHaveLength(1)
})

test('falls back to a default question when question is not a string', () => {
  render(MessageItem as Component, {
    props: { message: askHumanMessage({ question: 42, options: ['Yes'] }) }
  })

  expect(document.querySelector('.ask-human-question')?.textContent).toBe(
    'Please make a selection:'
  )
})

test('ask_human submit is a no-op without an onSubmitToolResult handler', async () => {
  render(MessageItem as Component, {
    props: {
      message: askHumanMessage({ question: 'Proceed?', options: ['Yes'] })
    }
  })

  await fireEvent.click(screen.getByDisplayValue('Yes'))
  await fireEvent.click(screen.getByRole('button', { name: 'Submit' }))
  // Still rendered, nothing thrown
  expect(document.querySelector('.ask-human-container')).toBeTruthy()
})

test('hides the ask_human form once the tool reports a result', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        ...askHumanMessage({ question: 'Proceed?', options: ['Yes'] }),
        content: '✅ ask_human completed'
      }
    }
  })

  expect(document.querySelector('.ask-human-container')).toBeNull()
  expect(document.querySelector('.tool-indicator')).toHaveClass('success')
})

/* -------------------------------------------------------------------------- */
/* options dropdown: copy / quote / outside click                             */
/* -------------------------------------------------------------------------- */

const openDropdown = async () => {
  await fireEvent.click(screen.getByTitle('Message options'))
}

test('copies string message content to the clipboard and closes the menu', async () => {
  render(MessageItem as Component, {
    props: {
      message: { id: 'c1', role: 'user', content: 'copy me', timestamp: 1 }
    }
  })

  await openDropdown()
  await fireEvent.click(screen.getByText('Copy'))

  expect(clipboardWrite).toHaveBeenCalledWith('copy me')
  await waitFor(() => expect(screen.queryByText('Copy')).toBeNull())
})

test('copies only the text parts of array content', async () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'c2',
        role: 'user',
        content: [
          { type: 'text', text: 'first' },
          { type: 'image_url', image_url: { url: 'http://x/y.png' } },
          { type: 'text', text: 'second' }
        ],
        timestamp: 1
      }
    }
  })

  await openDropdown()
  await fireEvent.click(screen.getByText('Copy'))

  expect(clipboardWrite).toHaveBeenCalledWith('first\nsecond\n')
})

test('logs an error when copying to the clipboard fails', async () => {
  clipboardWrite.mockRejectedValueOnce(new Error('denied'))

  render(MessageItem as Component, {
    props: {
      message: { id: 'c3', role: 'user', content: 'copy me', timestamp: 1 }
    }
  })

  await openDropdown()
  await fireEvent.click(screen.getByText('Copy'))

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to copy message:',
      expect.any(Error)
    )
  })
  // The menu stays open because closeDropdown is never reached
  expect(screen.queryByText('Copy')).not.toBeNull()
})

test('quotes the message through onQuote with trimmed text', async () => {
  const onQuote = vi.fn()

  render(MessageItem as Component, {
    props: {
      message: {
        id: 'q1',
        role: 'assistant',
        content: [
          { type: 'text', text: 'line one' },
          { type: 'text', text: 'line two' }
        ],
        timestamp: 1
      },
      onQuote
    }
  })

  await openDropdown()
  await fireEvent.click(screen.getByText('Quote'))

  expect(onQuote).toHaveBeenCalledWith('line one\nline two')
  await waitFor(() => expect(screen.queryByText('Quote')).toBeNull())
})

test('quoting without an onQuote handler just closes the menu', async () => {
  render(MessageItem as Component, {
    props: {
      message: { id: 'q2', role: 'user', content: 'no handler', timestamp: 1 }
    }
  })

  await openDropdown()
  await fireEvent.click(screen.getByText('Quote'))

  await waitFor(() => expect(screen.queryByText('Quote')).toBeNull())
})

test('closes the options dropdown on an outside click', async () => {
  render(MessageItem as Component, {
    props: {
      message: { id: 'd1', role: 'user', content: 'hi', timestamp: 1 }
    }
  })

  await openDropdown()
  expect(screen.queryByText('Copy')).not.toBeNull()

  await fireEvent.click(document.body)
  expect(screen.queryByText('Copy')).toBeNull()
})

test('keeps the dropdown open when the click lands inside the actions area', async () => {
  render(MessageItem as Component, {
    props: {
      message: { id: 'd2', role: 'user', content: 'hi', timestamp: 1 }
    }
  })

  await openDropdown()
  await fireEvent.click(document.querySelector('.message-actions')!)

  expect(screen.queryByText('Copy')).not.toBeNull()
})

/* -------------------------------------------------------------------------- */
/* code block copy buttons                                                    */
/* -------------------------------------------------------------------------- */

test('adds a copy button to completed code blocks and copies the code', async () => {
  vi.useFakeTimers()

  render(MessageItem as Component, {
    props: {
      message: {
        id: 'cb1',
        role: 'assistant',
        content: '```js\nconst a = 1\n```',
        timestamp: 1700000000
      }
    }
  })

  // Buttons are only injected after the post-stream settle delay
  expect(document.querySelector('.code-copy-button')).toBeNull()
  vi.advanceTimersByTime(250)

  const pre = document.querySelector('pre')!
  expect(pre).toHaveClass('has-copy-button')
  const button = pre.querySelector('.code-copy-button') as HTMLElement
  expect(button).toBeTruthy()
  expect(button.getAttribute('aria-label')).toBe('Copy code')
  expect(button.innerHTML).toContain(iconPath('content-copy'))

  button.dispatchEvent(new MouseEvent('click'))
  await vi.advanceTimersByTimeAsync(0)

  expect(clipboardWrite).toHaveBeenCalledWith('const a = 1\n')
  // Swaps to a checkmark as visual feedback...
  expect(button.innerHTML).toContain(iconPath('check'))

  // ...and reverts to the copy icon afterwards
  await vi.advanceTimersByTimeAsync(2100)
  expect(button.innerHTML).toContain(iconPath('content-copy'))
  expect(button.innerHTML).not.toContain(iconPath('check'))
})

test('hover handlers adjust the copy button opacity', () => {
  vi.useFakeTimers()

  render(MessageItem as Component, {
    props: {
      message: {
        id: 'cb2',
        role: 'assistant',
        content: '```\nplain\n```',
        timestamp: 1700000000
      }
    }
  })
  vi.advanceTimersByTime(250)

  const button = document.querySelector('.code-copy-button') as HTMLElement
  expect(button.style.opacity).toBe('0.7')
  button.dispatchEvent(new MouseEvent('mouseenter'))
  expect(button.style.opacity).toBe('1')
  button.dispatchEvent(new MouseEvent('mouseleave'))
  expect(button.style.opacity).toBe('0.7')
})

test('logs an error when copying a code block fails', async () => {
  vi.useFakeTimers()
  clipboardWrite.mockRejectedValue(new Error('nope'))

  render(MessageItem as Component, {
    props: {
      message: {
        id: 'cb3',
        role: 'assistant',
        content: '```\nboom\n```',
        timestamp: 1700000000
      }
    }
  })
  vi.advanceTimersByTime(250)

  const button = document.querySelector('.code-copy-button') as HTMLElement
  button.dispatchEvent(new MouseEvent('click'))
  await vi.advanceTimersByTimeAsync(0)

  expect(console.error).toHaveBeenCalledWith(
    'Failed to copy code:',
    expect.any(Error)
  )
  // No checkmark feedback on failure
  expect(button.innerHTML).toContain(iconPath('content-copy'))
})

test('skips pre blocks that contain no code element', () => {
  vi.useFakeTimers()

  render(MessageItem as Component, {
    props: {
      message: {
        id: 'cb4',
        role: 'assistant',
        content: '<pre>raw block</pre>',
        timestamp: 1700000000
      }
    }
  })
  vi.advanceTimersByTime(250)

  const pre = document.querySelector('pre')!
  expect(pre).toHaveClass('has-copy-button')
  expect(pre.querySelector('.code-copy-button')).toBeNull()
})

test('does not inject copy buttons while the assistant message is streaming', () => {
  vi.useFakeTimers()

  render(MessageItem as Component, {
    props: {
      message: {
        id: 'cb5',
        role: 'assistant',
        content: '```\nstreaming\n```',
        timestamp: 0
      }
    }
  })
  vi.advanceTimersByTime(500)

  expect(document.querySelector('.code-copy-button')).toBeNull()
})

/* -------------------------------------------------------------------------- */
/* charts                                                                     */
/* -------------------------------------------------------------------------- */

test('renders a chart plus the surrounding text for a json-chart block', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'ch1',
        role: 'assistant',
        content: `Before chart\n\n${chartFence('Sales')}\n\nAfter chart`,
        timestamp: 1
      }
    }
  })

  expect(document.querySelectorAll('.chart-container')).toHaveLength(1)
  const content = document.querySelector('.message-content')!
  expect(content.textContent).toContain('Before chart')
  expect(content.textContent).toContain('After chart')
  // The fence itself is consumed, not printed
  expect(content.textContent).not.toContain('json-chart')

  const config = chartMock.configs.at(-1)
  expect(config.type).toBe('bar')
  expect(config.data.labels).toEqual(['a', 'b'])
  expect(config.data.datasets[0].data).toEqual([1, 2])
})

test('renders a chart-only message without any stray text nodes', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'ch2',
        role: 'assistant',
        content: chartFence('Only'),
        timestamp: 1
      }
    }
  })

  expect(document.querySelectorAll('.chart-container')).toHaveLength(1)
  expect(document.querySelector('.message-content p')).toBeNull()
})

test('renders charts inside array content parts', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'ch3',
        role: 'assistant',
        content: [{ type: 'text', text: `Chart:\n\n${chartFence('Arr')}` }],
        timestamp: 1
      }
    }
  })

  expect(document.querySelectorAll('.chart-container')).toHaveLength(1)
  expect(chartMock.configs.at(-1).options.plugins.title.text).toBe('Arr')
})

test('renders charts and images while streaming array content', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'ch4',
        role: 'assistant',
        content: [
          { type: 'text', text: `Streaming chart\n\n${chartFence('Live')}` },
          { type: 'image_url', image_url: { url: 'http://x/stream.png' } }
        ],
        timestamp: 0
      }
    }
  })

  expect(document.querySelector('.message')).toHaveClass('streaming')
  expect(document.querySelector('.typing-indicator-inline')).toBeTruthy()
  expect(document.querySelectorAll('.chart-container')).toHaveLength(1)
  expect(document.querySelector('img.message-image')?.getAttribute('src')).toBe(
    'http://x/stream.png'
  )
})

test('renders charts while streaming string content', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'ch5',
        role: 'assistant',
        content: `Partial\n\n${chartFence('Streamed')}`,
        timestamp: 0
      }
    }
  })

  expect(document.querySelectorAll('.chart-container')).toHaveLength(1)
  expect(document.querySelector('.message-content')?.textContent).toContain(
    'Partial'
  )
  // Streaming content is not given the markdown class
  expect(document.querySelector('.message-content')).not.toHaveClass('markdown')
})

test('falls back to plain text when the chart payload is not valid JSON', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'ch6',
        role: 'assistant',
        content: '```json-chart\n{ not json }\n```',
        timestamp: 1
      }
    }
  })

  expect(console.error).toHaveBeenCalledWith(
    'Failed to parse chart data:',
    expect.any(Error)
  )
  expect(document.querySelector('.chart-container')).toBeNull()
  expect(chartMock.configs).toHaveLength(0)
  expect(document.querySelector('.message-content')?.textContent).toContain(
    'not json'
  )
})

/* -------------------------------------------------------------------------- */
/* misc content states                                                        */
/* -------------------------------------------------------------------------- */

test('renders the typing indicator for a streaming assistant message', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'm1',
        role: 'assistant',
        content: 'partial answ',
        timestamp: 0
      }
    }
  })

  expect(document.querySelector('.message')).toHaveClass('streaming')
  expect(
    document.querySelectorAll('.typing-indicator-inline span')
  ).toHaveLength(3)
  expect(document.querySelector('.message-content')?.textContent).toContain(
    'partial answ'
  )
})

test('renders only attachments for the "Sent files" placeholder', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'm2',
        role: 'user',
        content: 'Sent files',
        timestamp: 1,
        attachments: [{ name: 'notes.pdf', type: 'pdf' }]
      }
    }
  })

  expect(screen.getByText('notes.pdf')).toBeTruthy()
  expect(document.querySelector('.message-content')?.textContent).not.toContain(
    'Sent files'
  )
  expect(renderedIconPaths()).toContain(iconPath('file-pdf-box'))
})

test('renders an icon per attachment type', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'm3',
        role: 'user',
        content: '',
        timestamp: 1,
        attachments: [
          { name: 'a.txt', type: 'text' },
          { name: 'b.pdf', type: 'pdf' },
          { name: 'c.wav', type: 'audio' },
          { name: 'd.bin', type: 'something-else' }
        ]
      }
    }
  })

  expect(document.querySelectorAll('.attachment-icon')).toHaveLength(4)
  const paths = renderedIconPaths()
  expect(paths).toContain(iconPath('note-text'))
  expect(paths).toContain(iconPath('file-pdf-box'))
  expect(paths).toContain(iconPath('microphone'))
  expect(paths).toContain(iconPath('file'))
})

test('renders the assistant role marker and robot icon', () => {
  render(MessageItem as Component, {
    props: {
      message: {
        id: 'm4',
        role: 'assistant',
        content: 'hello',
        timestamp: 1
      }
    }
  })

  expect(screen.getByText('Assistant')).toBeTruthy()
  expect(document.querySelector('.message')).toHaveClass('assistant')
  expect(document.querySelector('svg.role-icon')).toBeTruthy()
  expect(renderedIconPaths()).toContain(iconPath('robot'))
})

test('renders no message body for empty content', () => {
  render(MessageItem as Component, {
    props: {
      message: { id: 'm5', role: 'user', content: '', timestamp: 1 }
    }
  })

  expect(document.querySelector('.message-content')?.textContent?.trim()).toBe(
    ''
  )
  expect(document.querySelector('.attachments-display')).toBeNull()
})
