/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import ChatInput from './ChatInput.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import type { Component } from 'svelte'
import { icons } from '@iconify-json/mdi'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    post: vi.fn()
  }
}))

// heic2any is dynamically imported by the image branch
const heicMock = vi.hoisted(() => ({ convert: vi.fn() }))
vi.mock('heic2any', () => ({ default: heicMock.convert }))

const mockedAxios = axiosBackendInstance as unknown as {
  post: ReturnType<typeof vi.fn>
}

const iconPath = (name: string): string =>
  icons.icons[name].body.match(/d="([^"]+)"/)![1]

/* ---------------------------------- DOM helpers --------------------------- */

const textarea = () =>
  document.querySelector('textarea.chat-input') as HTMLTextAreaElement

const sendButton = () =>
  document.querySelector('button.send-button') as HTMLElement

const fileInput = (accept: string) =>
  Array.from(document.querySelectorAll('input[type="file"]')).find((i) =>
    (i as HTMLInputElement).accept.includes(accept)
  ) as HTMLInputElement

const selectFile = async (accept: string, file: File) => {
  const input = fileInput(accept)
  expect(input).toBeTruthy()
  Object.defineProperty(input, 'files', { value: [file], configurable: true })
  await fireEvent.change(input)
  return input
}

// jsdom's Blob has no `.text()`, which the component relies on for text files.
const textFile = (name: string, content: string) => {
  const file = new File([content], name, { type: 'text/plain' })
  Object.defineProperty(file, 'text', {
    value: vi.fn().mockResolvedValue(content),
    writable: true
  })
  return file
}

const baseProps = () => ({
  inputMessage: '',
  loading: false,
  onSend: vi.fn(),
  onInputChange: vi.fn()
})

/* ----------------------------- image processing mocks --------------------- */

const imageState = {
  width: 100,
  height: 100,
  fail: false
}

class MockImage {
  onload: (() => void) | null = null
  onerror: (() => void) | null = null
  width = imageState.width
  height = imageState.height
  private _src = ''
  set src(value: string) {
    this._src = value
    Promise.resolve().then(() => {
      if (imageState.fail) this.onerror?.()
      else this.onload?.()
    })
  }
  get src() {
    return this._src
  }
}

const ctxMock = {
  fillStyle: '',
  fillRect: vi.fn(),
  drawImage: vi.fn()
}

let originalImage: typeof window.Image

const installImageMocks = (contextAvailable = true) => {
  originalImage = window.Image
  window.Image = MockImage as unknown as typeof window.Image
  ctxMock.fillRect = vi.fn()
  ctxMock.drawImage = vi.fn()
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(
    (contextAvailable ? ctxMock : null) as never
  )
  vi.spyOn(HTMLCanvasElement.prototype, 'toDataURL').mockReturnValue(
    'data:image/jpeg;base64,processed'
  )
}

/* ------------------------------ speech recognition ------------------------ */

class MockSpeechRecognition {
  static instances: MockSpeechRecognition[] = []
  continuous = false
  interimResults = false
  lang = ''
  onstart: (() => void) | null = null
  onend: (() => void) | null = null
  onerror: ((_e: unknown) => void) | null = null
  onresult: ((_e: unknown) => void) | null = null
  constructor() {
    MockSpeechRecognition.instances.push(this)
  }
  start() {
    this.onstart?.()
  }
  stop() {
    this.onend?.()
  }
  emitFinal(transcript: string) {
    this.onresult?.({
      resultIndex: 0,
      results: [{ isFinal: true, 0: { transcript } }]
    })
  }
}

const installSpeechRecognition = () => {
  MockSpeechRecognition.instances = []
  ;(window as unknown as Record<string, unknown>).SpeechRecognition =
    MockSpeechRecognition
}

beforeEach(() => {
  vi.clearAllMocks()
  imageState.width = 100
  imageState.height = 100
  imageState.fail = false
  vi.spyOn(console, 'log').mockImplementation(() => {})
  vi.spyOn(console, 'error').mockImplementation(() => {})
})

afterEach(() => {
  vi.restoreAllMocks()
  if (originalImage) {
    window.Image = originalImage
    originalImage = undefined as unknown as typeof window.Image
  }
  delete (window as unknown as Record<string, unknown>).SpeechRecognition
})

test('handles file selection', async () => {
  const onAttachmentsChange = vi.fn()

  // Mock File and FileReader
  const file = new File(['test content'], 'test.txt', { type: 'text/plain' })
  // Mock .text() method which is missing in some jsdom versions
  Object.defineProperty(file, 'text', {
    value: vi.fn().mockResolvedValue('test content'),
    writable: true
  })

  render(ChatInput as Component, {
    props: {
      inputMessage: '',
      loading: false,
      onSend: vi.fn(),
      onInputChange: vi.fn(),
      onAttachmentsChange
    }
  })

  const inputs = document.querySelectorAll('input[type="file"]')
  const textInput = Array.from(inputs).find((i) =>
    (i as HTMLInputElement).accept.includes('.txt')
  ) as HTMLInputElement

  expect(textInput).toBeTruthy()

  // Directly set files property on element to mock selection
  Object.defineProperty(textInput, 'files', {
    value: [file]
  })

  await fireEvent.change(textInput)

  await waitFor(() => {
    expect(onAttachmentsChange).toHaveBeenCalled()
    // Check if called with array containing the file
    const callArgs = onAttachmentsChange.mock.calls[0][0]
    expect(callArgs[0].name).toBe('test.txt')
    expect(callArgs[0].content).toBe('test content')
  })
})

test('handles PDF conversion mock', async () => {
  const onAttachmentsChange = vi.fn()

  mockedAxios.post.mockResolvedValueOnce({
    data: { markdown: 'Converted PDF content', filename: 'test.pdf' }
  })

  const file = new File(['%PDF...'], 'test.pdf', { type: 'application/pdf' })

  render(ChatInput as Component, {
    props: {
      inputMessage: '',
      loading: false,
      onSend: vi.fn(),
      onInputChange: vi.fn(),
      onAttachmentsChange
    }
  })

  const inputs = document.querySelectorAll('input[type="file"]')
  const pdfInput = Array.from(inputs).find((i) =>
    (i as HTMLInputElement).accept.includes('.pdf')
  ) as HTMLInputElement

  Object.defineProperty(pdfInput, 'files', {
    value: [file]
  })

  await fireEvent.change(pdfInput)

  await waitFor(() => {
    // Debug: check if log occurred
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'pdf-to-markdown',
      expect.any(FormData),
      expect.any(Object)
    )
    expect(onAttachmentsChange).toHaveBeenCalled()
    const callArgs = onAttachmentsChange.mock.calls[0][0]
    expect(callArgs[0].content).toBe('Converted PDF content')
  })
})

test('renders token usage when provided', async () => {
  const onAttachmentsChange = vi.fn()
  const tokenUsage = {
    prompt_tokens: 100,
    completion_tokens: 50,
    total_tokens: 150
  }

  const { queryByText } = render(ChatInput as Component, {
    props: {
      inputMessage: '',
      loading: false,
      onSend: vi.fn(),
      onInputChange: vi.fn(),
      onAttachmentsChange,
      tokenUsage,
      ctxSize: 200
    }
  })

  // Should show "150 / 200 tokens (75%)"
  expect(queryByText(/150 \/ 200 tokens/)).toBeTruthy()
})

test('does not render token usage when zero or null', async () => {
  const onAttachmentsChange = vi.fn()

  // Case 1: Null
  const { queryByText: queryByTextNull, unmount } = render(
    ChatInput as Component,
    {
      props: {
        inputMessage: '',
        loading: false,
        onSend: vi.fn(),
        onInputChange: vi.fn(),
        onAttachmentsChange,
        tokenUsage: null,
        ctxSize: 200
      }
    }
  )

  expect(queryByTextNull(/tokens/)).toBeNull()
  unmount()

  // Case 2: Zero
  const { queryByText: queryByTextZero } = render(ChatInput as Component, {
    props: {
      inputMessage: '',
      loading: false,
      onSend: vi.fn(),
      onInputChange: vi.fn(),
      onAttachmentsChange,
      tokenUsage: {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0
      },
      ctxSize: 200
    }
  })

  expect(queryByTextZero(/tokens/)).toBeNull()
})

/* -------------------------------------------------------------------------- */
/* submitting                                                                 */
/* -------------------------------------------------------------------------- */

test('sends the message when Enter is pressed', async () => {
  const props = { ...baseProps(), inputMessage: 'hello', onClearQuote: vi.fn() }
  render(ChatInput as Component, { props })

  await fireEvent.keyPress(textarea(), { key: 'Enter', charCode: 13 })

  expect(props.onSend).toHaveBeenCalledTimes(1)
  // Nothing to clean and no quote, so the parent is not asked to rewrite input
  expect(props.onInputChange).not.toHaveBeenCalled()
  expect(props.onClearQuote).toHaveBeenCalledTimes(1)
})

test('does not send on Shift+Enter or other keys', async () => {
  const props = { ...baseProps(), inputMessage: 'hello' }
  render(ChatInput as Component, { props })

  await fireEvent.keyPress(textarea(), {
    key: 'Enter',
    charCode: 13,
    shiftKey: true
  })
  await fireEvent.keyPress(textarea(), { key: 'a', charCode: 97 })

  expect(props.onSend).not.toHaveBeenCalled()
})

test('prepends the quoted message as a blockquote when sending', async () => {
  const props = {
    ...baseProps(),
    inputMessage: 'my reply',
    quotedMessage: 'quoted line\nsecond line',
    onClearQuote: vi.fn()
  }
  render(ChatInput as Component, { props })

  await fireEvent.keyPress(textarea(), { key: 'Enter', charCode: 13 })

  expect(props.onInputChange).toHaveBeenCalledWith(
    '> quoted line\n> second line\n\nmy reply'
  )
  expect(props.onSend).toHaveBeenCalledTimes(1)
  expect(props.onClearQuote).toHaveBeenCalledTimes(1)
})

test('strips attachment references from the input before sending on Enter', async () => {
  const props = {
    ...baseProps(),
    inputMessage: 'check [pdf:report.pdf] please'
  }
  render(ChatInput as Component, { props })

  await fireEvent.keyPress(textarea(), { key: 'Enter', charCode: 13 })

  expect(props.onInputChange).toHaveBeenCalledWith('check  please')
  expect(props.onSend).toHaveBeenCalledTimes(1)
})

test('the send button cleans the input, sends and clears attachments', async () => {
  const onAttachmentsChange = vi.fn()
  const props = {
    ...baseProps(),
    inputMessage: 'plain [text:a.txt]',
    onAttachmentsChange
  }
  render(ChatInput as Component, { props })

  await fireEvent.click(sendButton())

  expect(props.onInputChange).toHaveBeenCalledWith('plain ')
  expect(props.onSend).toHaveBeenCalledTimes(1)
  expect(onAttachmentsChange).toHaveBeenCalledWith([])
})

test('the send button is disabled until there is text or an attachment', async () => {
  const { unmount } = render(ChatInput as Component, {
    props: { ...baseProps(), inputMessage: '   ' }
  })
  expect(sendButton()).toBeDisabled()
  unmount()

  render(ChatInput as Component, {
    props: { ...baseProps(), inputMessage: 'something' }
  })
  expect(sendButton()).not.toBeDisabled()
})

test('an attachment alone enables the send button', async () => {
  const onAttachmentsChange = vi.fn()
  render(ChatInput as Component, {
    props: { ...baseProps(), onAttachmentsChange }
  })

  expect(sendButton()).toBeDisabled()

  await selectFile('.txt', textFile('note.txt', 'note'))

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  expect(sendButton()).not.toBeDisabled()
})

/* -------------------------------------------------------------------------- */
/* loading / stop                                                             */
/* -------------------------------------------------------------------------- */

test('swaps the send button for a stop button while loading', async () => {
  const onStop = vi.fn()
  render(ChatInput as Component, {
    props: { ...baseProps(), loading: true, inputMessage: 'busy', onStop }
  })

  const stop = document.querySelector('button.stop-button') as HTMLElement
  expect(stop).toBeTruthy()
  expect(stop.title).toBe('Stop generation')
  expect(textarea()).toBeDisabled()
  expect(screen.getByTitle('Upload PDF file')).toBeDisabled()

  await fireEvent.click(stop)
  expect(onStop).toHaveBeenCalledTimes(1)
})

test('the stop button falls back to a no-op handler', async () => {
  render(ChatInput as Component, {
    props: { ...baseProps(), loading: true }
  })

  const stop = document.querySelector('button.stop-button') as HTMLElement
  await fireEvent.click(stop)
  expect(stop).toBeTruthy()
})

/* -------------------------------------------------------------------------- */
/* quote banner                                                               */
/* -------------------------------------------------------------------------- */

test('renders the quote banner and dismisses it', async () => {
  const onClearQuote = vi.fn()
  render(ChatInput as Component, {
    props: { ...baseProps(), quotedMessage: 'remember this', onClearQuote }
  })

  const banner = document.querySelector('.quote-banner')!
  expect(banner).toBeTruthy()
  expect(banner.textContent).toContain('remember this')

  await fireEvent.click(screen.getByLabelText('Dismiss quote'))
  expect(onClearQuote).toHaveBeenCalledTimes(1)
})

test('renders no quote banner without a quoted message', () => {
  render(ChatInput as Component, { props: baseProps() })
  expect(document.querySelector('.quote-banner')).toBeNull()
})

/* -------------------------------------------------------------------------- */
/* textarea behaviour                                                         */
/* -------------------------------------------------------------------------- */

test('auto-resizes the textarea up to a 150px ceiling', async () => {
  const scrollHeight = vi
    .spyOn(window.Element.prototype, 'scrollHeight', 'get')
    .mockReturnValue(80)

  const props = baseProps()
  render(ChatInput as Component, { props })

  await fireEvent.input(textarea(), { target: { value: 'one line' } })
  expect(props.onInputChange).toHaveBeenCalledWith('one line')
  expect(textarea().style.height).toBe('80px')

  scrollHeight.mockReturnValue(400)
  await fireEvent.input(textarea(), { target: { value: 'many\nlines' } })
  expect(textarea().style.height).toBe('150px')
})

/* -------------------------------------------------------------------------- */
/* attachment chips                                                           */
/* -------------------------------------------------------------------------- */

test('shows an attachment chip and removes it again', async () => {
  const onAttachmentsChange = vi.fn()
  const props = {
    ...baseProps(),
    inputMessage: 'see [text:note.txt]',
    onAttachmentsChange
  }
  render(ChatInput as Component, { props })

  expect(document.querySelector('.attachments-preview')).toBeNull()

  await selectFile('.txt', textFile('note.txt', 'note'))

  await waitFor(() => expect(screen.getByText('note.txt')).toBeTruthy())
  // The inline reference is stripped now that the file is a real attachment
  expect(props.onInputChange).toHaveBeenCalledWith('see ')
  expect(
    document.querySelector('.attachment-chip svg path')?.getAttribute('d')
  ).toBe(iconPath('note-text'))

  await fireEvent.click(screen.getByTitle('Remove attachment'))

  expect(screen.queryByText('note.txt')).toBeNull()
  expect(document.querySelector('.attachments-preview')).toBeNull()
  expect(onAttachmentsChange).toHaveBeenLastCalledWith([])
})

/* -------------------------------------------------------------------------- */
/* file pickers                                                               */
/* -------------------------------------------------------------------------- */

test('each upload button opens its matching hidden input', async () => {
  render(ChatInput as Component, {
    props: {
      ...baseProps(),
      modelCapabilities: { vision: true, audio: true }
    }
  })

  const clicks: Record<string, ReturnType<typeof vi.fn>> = {}
  for (const accept of ['.txt', '.pdf', 'audio/', 'image/']) {
    clicks[accept] = vi.fn()
    fileInput(accept).click = clicks[accept]
  }

  await fireEvent.click(screen.getByTitle('Upload text file (txt, md)'))
  expect(clicks['.txt']).toHaveBeenCalledTimes(1)

  await fireEvent.click(screen.getByTitle('Upload PDF file'))
  expect(clicks['.pdf']).toHaveBeenCalledTimes(1)

  await fireEvent.click(screen.getByTitle('Upload audio file'))
  expect(clicks['audio/']).toHaveBeenCalledTimes(1)

  await fireEvent.click(screen.getByTitle('Upload image file'))
  expect(clicks['image/']).toHaveBeenCalledTimes(1)
})

test('hides the audio and image pickers when the model lacks the capability', () => {
  render(ChatInput as Component, { props: baseProps() })

  expect(screen.queryByTitle('Upload audio file')).toBeNull()
  expect(screen.queryByTitle('Upload image file')).toBeNull()
  expect(document.querySelectorAll('input[type="file"]')).toHaveLength(2)
})

test('ignores a change event with no selected file', async () => {
  const onAttachmentsChange = vi.fn()
  render(ChatInput as Component, {
    props: { ...baseProps(), onAttachmentsChange }
  })

  const input = fileInput('.txt')
  Object.defineProperty(input, 'files', { value: [], configurable: true })
  await fireEvent.change(input)

  expect(onAttachmentsChange).not.toHaveBeenCalled()
  expect(document.querySelector('.attachments-preview')).toBeNull()
})

test('logs an error when a text file cannot be read', async () => {
  const onAttachmentsChange = vi.fn()
  const file = new File(['x'], 'broken.txt', { type: 'text/plain' })
  Object.defineProperty(file, 'text', {
    value: vi.fn().mockRejectedValue(new Error('read failed')),
    writable: true
  })

  render(ChatInput as Component, {
    props: { ...baseProps(), onAttachmentsChange }
  })
  await selectFile('.txt', file)

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to process text file:',
      expect.any(Error)
    )
  })
  expect(onAttachmentsChange).not.toHaveBeenCalled()
})

test('reports the backend error when PDF conversion fails', async () => {
  const onAttachmentsChange = vi.fn()
  mockedAxios.post.mockRejectedValueOnce({
    response: { data: { error: 'unsupported pdf' } }
  })

  render(ChatInput as Component, {
    props: { ...baseProps(), onAttachmentsChange }
  })
  await selectFile(
    '.pdf',
    new File(['%PDF'], 'broken.pdf', { type: 'application/pdf' })
  )

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  expect(onAttachmentsChange.mock.calls.at(-1)![0][0].content).toBe(
    'Failed to extract text: unsupported pdf'
  )
})

test('falls back to the error message when the PDF failure has no response body', async () => {
  const onAttachmentsChange = vi.fn()
  mockedAxios.post.mockRejectedValueOnce(new Error('network down'))

  render(ChatInput as Component, {
    props: { ...baseProps(), onAttachmentsChange }
  })
  await selectFile(
    '.pdf',
    new File(['%PDF'], 'broken.pdf', { type: 'application/pdf' })
  )

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  expect(onAttachmentsChange.mock.calls.at(-1)![0][0].content).toBe(
    'Failed to extract text: network down'
  )
})

test('detects a PDF picked through the text file picker', async () => {
  const onAttachmentsChange = vi.fn()
  mockedAxios.post.mockResolvedValueOnce({
    data: { markdown: '# from pdf', filename: 'sneaky.pdf' }
  })

  render(ChatInput as Component, {
    props: { ...baseProps(), onAttachmentsChange }
  })
  await selectFile(
    '.txt',
    new File(['%PDF'], 'sneaky.pdf', { type: 'application/pdf' })
  )

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  const attachment = onAttachmentsChange.mock.calls.at(-1)![0][0]
  expect(attachment.type).toBe('pdf')
  expect(attachment.content).toBe('# from pdf')
})

test('encodes an audio file as base64 and cleans the input', async () => {
  const onAttachmentsChange = vi.fn()
  const props = {
    ...baseProps(),
    inputMessage: 'listen [text:old.txt]',
    modelCapabilities: { vision: false, audio: true },
    onAttachmentsChange
  }

  render(ChatInput as Component, { props })
  await selectFile(
    'audio/',
    new File(['audio-bytes'], 'clip.wav', { type: 'audio/wav' })
  )

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  const attachment = onAttachmentsChange.mock.calls.at(-1)![0][0]
  expect(attachment.type).toBe('audio')
  expect(attachment.name).toBe('clip.wav')
  expect(window.atob(attachment.content)).toBe('audio-bytes')
  expect(props.onInputChange).toHaveBeenCalledWith('listen ')
  expect(screen.getByText('clip.wav')).toBeTruthy()
  expect(
    document.querySelector('.attachment-chip svg path')?.getAttribute('d')
  ).toBe(iconPath('microphone'))
})

/* -------------------------------------------------------------------------- */
/* image processing                                                           */
/* -------------------------------------------------------------------------- */

const imageProps = (onAttachmentsChange: ReturnType<typeof vi.fn>) => ({
  ...baseProps(),
  modelCapabilities: { vision: true, audio: false },
  onAttachmentsChange
})

test('resizes a wide image down to the max dimension and stores JPEG data', async () => {
  installImageMocks()
  imageState.width = 2000
  imageState.height = 1000

  const onAttachmentsChange = vi.fn()
  const props = {
    ...imageProps(onAttachmentsChange),
    inputMessage: 'look [image:photo.png]'
  }
  render(ChatInput as Component, { props })

  await selectFile(
    'image/',
    new File(['png-bytes'], 'photo.png', { type: 'image/png' })
  )

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  expect(props.onInputChange).toHaveBeenCalledWith('look ')
  const attachment = onAttachmentsChange.mock.calls.at(-1)![0][0]
  expect(attachment.type).toBe('image')
  expect(attachment.content).toBe('data:image/jpeg;base64,processed')
  // Non-jpeg names are rewritten to .jpg because the payload is now JPEG
  expect(attachment.name).toBe('photo.jpg')
  expect(ctxMock.drawImage).toHaveBeenCalledWith(
    expect.anything(),
    0,
    0,
    1536,
    768
  )
})

test('resizes a tall image against its height', async () => {
  installImageMocks()
  imageState.width = 800
  imageState.height = 2400

  const onAttachmentsChange = vi.fn()
  render(ChatInput as Component, { props: imageProps(onAttachmentsChange) })

  await selectFile(
    'image/',
    new File(['png-bytes'], 'tall.jpeg', { type: 'image/png' })
  )

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  // Already a jpeg-ish name, so it is left alone
  expect(onAttachmentsChange.mock.calls.at(-1)![0][0].name).toBe('tall.jpeg')
  expect(ctxMock.drawImage).toHaveBeenCalledWith(
    expect.anything(),
    0,
    0,
    512,
    1536
  )
})

test('leaves small images at their original size', async () => {
  installImageMocks()
  imageState.width = 120
  imageState.height = 90

  const onAttachmentsChange = vi.fn()
  render(ChatInput as Component, { props: imageProps(onAttachmentsChange) })

  await selectFile(
    'image/',
    new File(['png-bytes'], 'small.jpg', { type: 'image/png' })
  )

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  expect(ctxMock.drawImage).toHaveBeenCalledWith(
    expect.anything(),
    0,
    0,
    120,
    90
  )
})

test('converts HEIC images through heic2any before processing', async () => {
  installImageMocks()
  const blob = new Blob(['jpeg-bytes'], { type: 'image/jpeg' })
  heicMock.convert.mockResolvedValue(blob)

  const onAttachmentsChange = vi.fn()
  render(ChatInput as Component, { props: imageProps(onAttachmentsChange) })

  const file = new File(['heic-bytes'], 'holiday.HEIC', { type: '' })
  await selectFile('image/', file)

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  expect(heicMock.convert).toHaveBeenCalledWith({
    blob: file,
    toType: 'image/jpeg',
    quality: 0.85
  })
  const attachment = onAttachmentsChange.mock.calls.at(-1)![0][0]
  expect(attachment.type).toBe('image')
  expect(attachment.name).toBe('holiday.jpg')
  expect(attachment.content).toBe('data:image/jpeg;base64,processed')
})

test('accepts an array of blobs back from heic2any', async () => {
  installImageMocks()
  heicMock.convert.mockResolvedValue([
    new Blob(['jpeg-bytes'], { type: 'image/jpeg' })
  ])

  const onAttachmentsChange = vi.fn()
  render(ChatInput as Component, { props: imageProps(onAttachmentsChange) })

  await selectFile(
    'image/',
    new File(['heic-bytes'], 'burst.heif', { type: 'image/heif' })
  )

  await waitFor(() => expect(onAttachmentsChange).toHaveBeenCalled())
  expect(onAttachmentsChange.mock.calls.at(-1)![0][0].name).toBe('burst.jpg')
})

test('logs and skips the attachment when the canvas context is unavailable', async () => {
  installImageMocks(false)

  const onAttachmentsChange = vi.fn()
  render(ChatInput as Component, { props: imageProps(onAttachmentsChange) })

  await selectFile(
    'image/',
    new File(['png-bytes'], 'photo.png', { type: 'image/png' })
  )

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to process image:',
      expect.objectContaining({ message: 'Failed to get canvas context' })
    )
  })
  expect(onAttachmentsChange).not.toHaveBeenCalled()
  expect(document.querySelector('.attachment-chip')).toBeNull()
})

test('logs and skips the attachment when the image cannot be decoded', async () => {
  installImageMocks()
  imageState.fail = true

  const onAttachmentsChange = vi.fn()
  render(ChatInput as Component, { props: imageProps(onAttachmentsChange) })

  await selectFile(
    'image/',
    new File(['not-an-image'], 'photo.png', { type: 'image/png' })
  )

  await waitFor(() => {
    expect(console.error).toHaveBeenCalledWith(
      'Failed to process image:',
      expect.objectContaining({ message: 'Failed to load image' })
    )
  })
  expect(onAttachmentsChange).not.toHaveBeenCalled()
})

/* -------------------------------------------------------------------------- */
/* voice input + TTS controls                                                 */
/* -------------------------------------------------------------------------- */

test('appends the voice transcript to the existing input', async () => {
  installSpeechRecognition()
  const props = { ...baseProps(), inputMessage: 'hi' }
  render(ChatInput as Component, { props })

  await fireEvent.click(screen.getByTitle('Start Voice Input'))
  const recognition = MockSpeechRecognition.instances.at(-1)!
  recognition.emitFinal('hello there')

  expect(props.onInputChange).toHaveBeenCalledWith('hi hello there')
})

test('uses the transcript as-is when the input is empty', async () => {
  installSpeechRecognition()
  const props = baseProps()
  render(ChatInput as Component, { props })

  await fireEvent.click(screen.getByTitle('Start Voice Input'))
  MockSpeechRecognition.instances.at(-1)!.emitFinal('hello there')

  expect(props.onInputChange).toHaveBeenCalledWith('hello there')
})

test('a spoken send command cleans the input and submits', async () => {
  installSpeechRecognition()
  const props = { ...baseProps(), inputMessage: 'draft [text:a.txt]' }
  render(ChatInput as Component, { props })

  await fireEvent.click(screen.getByTitle('Start Voice Input'))
  MockSpeechRecognition.instances.at(-1)!.emitFinal('run it send')

  // The transcript (minus the command word) is handed over first
  expect(props.onInputChange).toHaveBeenCalledWith('draft [text:a.txt] run it')

  await waitFor(() => expect(props.onSend).toHaveBeenCalledTimes(1))
  expect(props.onInputChange).toHaveBeenLastCalledWith('draft ')
})

test('passes the selected language through to speech recognition', async () => {
  installSpeechRecognition()
  render(ChatInput as Component, {
    props: { ...baseProps(), language: 'pl-PL' }
  })

  await fireEvent.click(screen.getByTitle('Start Voice Input'))
  expect(MockSpeechRecognition.instances.at(-1)!.lang).toBe('pl-PL')
  expect((document.querySelector('select') as HTMLSelectElement).value).toBe(
    'pl-PL'
  )
})

test('hides the voice input when speech recognition is unsupported', () => {
  render(ChatInput as Component, { props: baseProps() })
  expect(screen.queryByTitle('Start Voice Input')).toBeNull()
})

test('renders the TTS toggle only when a handler is supplied', async () => {
  const onToggleTTS = vi.fn()
  const { unmount } = render(ChatInput as Component, {
    props: { ...baseProps(), onToggleTTS }
  })

  await fireEvent.click(screen.getByTitle('Read Messages: Off'))
  expect(onToggleTTS).toHaveBeenCalledTimes(1)
  unmount()

  render(ChatInput as Component, { props: baseProps() })
  expect(screen.queryByTitle('Read Messages: Off')).toBeNull()
})

test('the TTS button stops playback while speaking', async () => {
  const onToggleTTS = vi.fn()
  const onStopTTS = vi.fn()
  render(ChatInput as Component, {
    props: {
      ...baseProps(),
      ttsEnabled: true,
      ttsSpeaking: true,
      onToggleTTS,
      onStopTTS
    }
  })

  await fireEvent.click(screen.getByTitle('Stop Speaking'))
  expect(onStopTTS).toHaveBeenCalledTimes(1)
  expect(onToggleTTS).not.toHaveBeenCalled()
})
