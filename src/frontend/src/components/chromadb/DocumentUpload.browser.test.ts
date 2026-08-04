/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import DocumentUpload from './DocumentUpload.svelte'

// Mock axiosBackendInstance

const mockWebSocket = (success: boolean, message: string) => {
  return class MockWebSocket {
    onmessage: any
    onopen: any
    constructor(_url: string) {
      setTimeout(() => {
        if (this.onopen) {
          this.onopen()
        }
        if (this.onmessage) {
          const data = JSON.stringify({
            status: success ? 'completed' : 'error',
            message,
            success
          })
          this.onmessage({ data })
        }
      }, 50)
    }
    close() {}
  }
}

/**
 * A WebSocket stub the test drives by hand, so each individual
 * `onmessage` / `onopen` / `onerror` path can be exercised in isolation.
 */
type SocketStub = {
  url: string
  closed: boolean
  onmessage: ((_e: { data: string }) => void) | null
  onopen: (() => void) | null
  onerror: (() => void) | null
}

const useDrivableWebSocket = (): SocketStub[] => {
  const sockets: SocketStub[] = []
  class DrivableWebSocket {
    url: string
    closed = false
    onmessage: ((_e: { data: string }) => void) | null = null
    onopen: (() => void) | null = null
    onerror: (() => void) | null = null
    constructor(url: string) {
      this.url = url
      sockets.push(this as unknown as SocketStub)
    }
    close() {
      this.closed = true
    }
  }
  global.WebSocket = DrivableWebSocket as any
  return sockets
}

const emit = (socket: SocketStub, payload: unknown) =>
  socket.onmessage?.({
    data: typeof payload === 'string' ? payload : JSON.stringify(payload)
  })

const asFileList = (files: File[]) =>
  ({
    ...files,
    length: files.length,
    item: (index: number) => files[index] ?? null,
    [Symbol.iterator]: function* () {
      yield* files
    }
  }) as unknown as FileList

const attachFiles = async (files: File[]) => {
  const input = document.querySelector('input[type="file"]') as HTMLInputElement
  Object.defineProperty(input, 'files', {
    value: asFileList(files),
    writable: false,
    configurable: true
  })
  await fireEvent.change(input)
  await waitFor(() => {
    expect(screen.getByText(files[0].name)).toBeTruthy()
  })
}

const pdf = (name = 'test.pdf', content = 'test') =>
  new File([content], name, { type: 'application/pdf' })

beforeEach(() => {
  vi.clearAllMocks()
  global.fetch = vi.fn().mockResolvedValue({ ok: true })
  global.URL.createObjectURL = vi.fn(() => 'blob:mock-url')
  global.URL.revokeObjectURL = vi.fn()
  global.WebSocket = mockWebSocket(true, 'Success') as any
})

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllEnvs()
})

test('renders document upload component', () => {
  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' }
  })

  expect(screen.getByText('Upload Documents')).toBeTruthy()
})

test('shows warning when no collection is selected', () => {
  render(DocumentUpload, {
    props: { selectedCollection: null }
  })

  expect(
    screen.getByText('⚠️ Please select a collection first to upload documents')
  ).toBeTruthy()
})

test('handles file selection', async () => {
  const file = new File(['test content'], 'test.pdf', {
    type: 'application/pdf'
  })
  const fileList = {
    0: file,
    length: 1,
    item: (index: number) => (index === 0 ? file : null),
    [Symbol.iterator]: function* () {
      yield file
    }
  } as FileList

  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' }
  })

  const input = document.querySelector('input[type="file"]') as HTMLInputElement

  Object.defineProperty(input, 'files', {
    value: fileList,
    writable: false
  })

  fireEvent.change(input)

  await waitFor(() => {
    expect(screen.getByText('test.pdf')).toBeTruthy()
  })
})

test('removes file from list', async () => {
  const file = new File(['test'], 'test.pdf', { type: 'application/pdf' })
  const fileList = {
    0: file,
    length: 1,
    item: (index: number) => (index === 0 ? file : null),
    [Symbol.iterator]: function* () {
      yield file
    }
  } as FileList

  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' }
  })

  const input = document.querySelector('input[type="file"]') as HTMLInputElement
  Object.defineProperty(input, 'files', {
    value: fileList,
    writable: false
  })

  fireEvent.change(input)

  await waitFor(() => {
    expect(screen.getByText('test.pdf')).toBeTruthy()
  })

  const removeButton = screen.getByTitle('Remove file')
  fireEvent.click(removeButton)

  await waitFor(() => {
    expect(screen.queryByText('test.pdf')).not.toBeInTheDocument()
  })
})

test('uploads documents successfully', async () => {
  global.WebSocket = mockWebSocket(true, 'Upload successful') as any

  const file = new File(['test'], 'test.pdf', { type: 'application/pdf' })
  const fileList = {
    0: file,
    length: 1,
    item: (index: number) => (index === 0 ? file : null),
    [Symbol.iterator]: function* () {
      yield file
    }
  } as FileList

  const handleUploaded = vi.fn()
  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' },
    events: { uploaded: handleUploaded }
  })

  const input = document.querySelector('input[type="file"]') as HTMLInputElement
  Object.defineProperty(input, 'files', {
    value: fileList,
    writable: false
  })

  fireEvent.change(input)

  await waitFor(() => {
    expect(screen.getByText('test.pdf')).toBeTruthy()
  })

  const uploadButton = screen.getByText('Upload 1 file')
  fireEvent.click(uploadButton)

  await waitFor(() => {
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('chromadb/documents/upload'),
      expect.objectContaining({
        method: 'POST',
        body: expect.any(FormData)
      })
    )
  })

  await waitFor(() => {
    expect(handleUploaded).toHaveBeenCalled()
  })
})

test('shows error when upload fails', async () => {
  global.WebSocket = mockWebSocket(false, 'Upload failed') as any

  const file = new File(['test'], 'test.pdf', { type: 'application/pdf' })
  const fileList = {
    0: file,
    length: 1,
    item: (index: number) => (index === 0 ? file : null),
    [Symbol.iterator]: function* () {
      yield file
    }
  } as FileList

  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' }
  })

  const input = document.querySelector('input[type="file"]') as HTMLInputElement
  Object.defineProperty(input, 'files', {
    value: fileList,
    writable: false
  })

  fireEvent.change(input)

  await waitFor(() => {
    expect(screen.getByText('test.pdf')).toBeTruthy()
  })

  const uploadButton = screen.getByText('Upload 1 file')
  fireEvent.click(uploadButton)

  await waitFor(() => {
    expect(screen.getByText(/Upload failed/)).toBeTruthy()
  })
})

test('filters invalid file types', async () => {
  const invalidFile = new File(['test'], 'test.exe', {
    type: 'application/x-msdownload'
  })
  const fileList = {
    0: invalidFile,
    length: 1,
    item: (index: number) => (index === 0 ? invalidFile : null),
    [Symbol.iterator]: function* () {
      yield invalidFile
    }
  } as FileList

  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' }
  })

  const input = document.querySelector('input[type="file"]') as HTMLInputElement
  Object.defineProperty(input, 'files', {
    value: fileList,
    writable: false
  })

  fireEvent.change(input)

  // Invalid file should not be added
  await new Promise((resolve) => setTimeout(resolve, 100))
  expect(screen.queryByText('test.exe')).not.toBeInTheDocument()
})

test('disables upload button when no files selected', () => {
  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' }
  })

  // Upload button should not be visible when no files
  expect(screen.queryByText(/Upload \d+ file/)).not.toBeInTheDocument()
})

test('displays file size correctly', async () => {
  const file = new File(['x'.repeat(1024)], 'test.pdf', {
    type: 'application/pdf'
  })
  const fileList = {
    0: file,
    length: 1,
    item: (index: number) => (index === 0 ? file : null),
    [Symbol.iterator]: function* () {
      yield file
    }
  } as FileList

  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' }
  })

  const input = document.querySelector('input[type="file"]') as HTMLInputElement
  Object.defineProperty(input, 'files', {
    value: fileList,
    writable: false
  })

  fireEvent.change(input)

  await waitFor(() => {
    expect(screen.getByText(/1 KB/)).toBeTruthy()
  })
})

test('formats an empty file as 0 Bytes', async () => {
  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })

  await attachFiles([new File([], 'empty.txt', { type: 'text/plain' })])

  expect(screen.getByText('0 Bytes')).toBeTruthy()
})

test('pluralises the upload button label for several files', async () => {
  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })

  await attachFiles([pdf('a.pdf'), pdf('b.pdf', 'other')])

  expect(screen.getByText('Selected Files (2)')).toBeTruthy()
  expect(screen.getByText('Upload 2 files')).toBeTruthy()
})

test('reports partially skipped selections but keeps the valid files', async () => {
  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })

  const input = document.querySelector('input[type="file"]') as HTMLInputElement
  Object.defineProperty(input, 'files', {
    value: asFileList([
      pdf('keep.pdf'),
      new File(['x'], 'drop.exe', { type: 'application/x-msdownload' })
    ]),
    writable: false,
    configurable: true
  })
  await fireEvent.change(input)

  await waitFor(() => {
    expect(
      screen.getByText(
        'Some files were skipped. Only PDF, Markdown, and text files are supported.'
      )
    ).toBeTruthy()
  })
  expect(screen.getByText('keep.pdf')).toBeTruthy()
  expect(screen.queryByText('drop.exe')).not.toBeInTheDocument()
})

test('clears the pending files when the collection changes', async () => {
  const { rerender } = render(DocumentUpload, {
    props: { selectedCollection: 'first' }
  })

  await attachFiles([pdf('keep-me.pdf')])

  await rerender({ selectedCollection: 'second' })

  await waitFor(() => {
    expect(screen.queryByText('keep-me.pdf')).not.toBeInTheDocument()
  })
})

test('opens the log websocket against the backend and posts the form data', async () => {
  const sockets = useDrivableWebSocket()

  render(DocumentUpload, { props: { selectedCollection: 'my-collection' } })
  await attachFiles([pdf()])

  await fireEvent.click(screen.getByText('Upload 1 file'))

  expect(sockets).toHaveLength(1)
  expect(sockets[0].url).toMatch(/^ws:\/\/.+\/chromadb\/logs\/ws$/)
  // nothing is posted until the socket is open
  expect(global.fetch).not.toHaveBeenCalled()
  expect(screen.getByText('Preparing files...')).toBeTruthy()

  sockets[0].onopen?.()

  await waitFor(() => {
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/chromadb/documents/upload'),
      expect.objectContaining({ method: 'POST', body: expect.any(FormData) })
    )
  })

  const body = (global.fetch as any).mock.calls[0][1].body as FormData
  expect(body.get('collection')).toBe('my-collection')
  expect((body.get('files') as File).name).toBe('test.pdf')
})

test('uses a secure websocket when the backend is served over https', async () => {
  vi.stubEnv('PUBLIC_API_URL', 'https://chroma.example.com/api/')
  const sockets = useDrivableWebSocket()

  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  expect(sockets[0].url).toBe('wss://chroma.example.com/api/chromadb/logs/ws')

  sockets[0].onopen?.()

  await waitFor(() => {
    expect(global.fetch).toHaveBeenCalledWith(
      'https://chroma.example.com/api/chromadb/documents/upload',
      expect.anything()
    )
  })
})

test('streams log lines into the log panel while processing', async () => {
  const sockets = useDrivableWebSocket()

  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  emit(sockets[0], { status: 'log', message: 'parsing test.pdf' })
  emit(sockets[0], { status: 'log', message: 'embedding chunk 1' })

  await waitFor(() => {
    const container = document.querySelector('.logs-container')
    expect(container).not.toBeNull()
    expect(container?.textContent).toContain('parsing test.pdf')
    expect(container?.textContent).toContain('embedding chunk 1')
  })
  expect(document.querySelectorAll('.log-line')).toHaveLength(2)
  // let the queued auto-scroll timer run against the rendered container
  await new Promise((resolve) => setTimeout(resolve, 5))
  // log messages must not overwrite the status line
  expect(screen.getByText('Preparing files...')).toBeTruthy()
})

test('reflects progress counters pushed over the websocket', async () => {
  const sockets = useDrivableWebSocket()

  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })
  await attachFiles([pdf('a.pdf'), pdf('b.pdf', 'other')])
  await fireEvent.click(screen.getByText('Upload 2 files'))

  emit(sockets[0], {
    status: 'processing',
    message: 'Embedding documents',
    processed_files: 1,
    total_files: 2
  })

  await waitFor(() => {
    expect(screen.getByText('Embedding documents')).toBeTruthy()
  })
  expect(screen.getByText('1 / 2 files processed')).toBeTruthy()
  expect(
    document.querySelector('.progress-fill')?.getAttribute('style')
  ).toContain('width: 100%')
})

test('completes the upload, clears the files and notifies the parent', async () => {
  const sockets = useDrivableWebSocket()
  const handleUploaded = vi.fn()

  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' },
    events: { uploaded: handleUploaded }
  })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  emit(sockets[0], { status: 'completed', message: 'All documents indexed' })

  await waitFor(() => {
    expect(handleUploaded).toHaveBeenCalledTimes(1)
  })
  expect(handleUploaded.mock.calls[0][0].detail).toEqual({
    collection: 'test-collection',
    files: 1
  })
  expect(sockets[0].closed).toBe(true)

  await waitFor(() => {
    expect(screen.queryByText('test.pdf')).not.toBeInTheDocument()
  })
  expect(screen.getByText('All documents indexed')).toBeTruthy()
  expect(document.querySelector('.status')).toHaveClass('completed')
})

test('shows an error status when the websocket reports a failure', async () => {
  const sockets = useDrivableWebSocket()
  const handleUploaded = vi.fn()

  render(DocumentUpload, {
    props: { selectedCollection: 'test-collection' },
    events: { uploaded: handleUploaded }
  })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  emit(sockets[0], {
    status: 'error',
    success: false,
    message: 'embedding model missing'
  })

  await waitFor(() => {
    expect(screen.getByText('embedding model missing')).toBeTruthy()
  })
  expect(document.querySelector('.status')).toHaveClass('error')
  expect(document.querySelector('.progress-bar')).toBeNull()
  expect(sockets[0].closed).toBe(true)
  expect(handleUploaded).not.toHaveBeenCalled()
  // the selection is kept so the user can retry
  expect(screen.getByText('test.pdf')).toBeTruthy()
})

test('keeps the socket open for a non-fatal error frame', async () => {
  const sockets = useDrivableWebSocket()

  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  // success is not `false`, so this is treated as a plain status update
  emit(sockets[0], { status: 'error', message: 'retrying chunk' })

  await waitFor(() => {
    expect(screen.getByText('retrying chunk')).toBeTruthy()
  })
  expect(sockets[0].closed).toBe(false)
  expect(document.querySelector('.status')).toHaveClass('processing')
})

test('ignores malformed websocket frames', async () => {
  const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
  const sockets = useDrivableWebSocket()

  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  emit(sockets[0], '{ not json')

  await waitFor(() => {
    expect(errorSpy).toHaveBeenCalledWith(
      'Error parsing WS json',
      expect.any(Error)
    )
  })
  // the previous status is untouched
  expect(screen.getByText('Preparing files...')).toBeTruthy()
  expect(document.querySelector('.status')).toHaveClass('processing')
})

test('reports a websocket connection failure', async () => {
  vi.spyOn(console, 'error').mockImplementation(() => {})
  const sockets = useDrivableWebSocket()

  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  sockets[0].onerror?.()

  await waitFor(() => {
    expect(screen.getByText('WebSocket connection failed')).toBeTruthy()
  })
  expect(document.querySelector('.status')).toHaveClass('error')
  expect(global.fetch).not.toHaveBeenCalled()
})

test('reports a non-2xx upload response', async () => {
  vi.spyOn(console, 'error').mockImplementation(() => {})
  const sockets = useDrivableWebSocket()
  global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 503 })

  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  sockets[0].onopen?.()

  await waitFor(() => {
    expect(screen.getByText('HTTP error! status: 503')).toBeTruthy()
  })
  expect(document.querySelector('.status')).toHaveClass('error')
})

test('prefers the backend error payload when the upload request rejects', async () => {
  vi.spyOn(console, 'error').mockImplementation(() => {})
  const sockets = useDrivableWebSocket()
  global.fetch = vi
    .fn()
    .mockRejectedValue({ response: { data: { error: 'disk full' } } })

  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  sockets[0].onopen?.()

  await waitFor(() => {
    expect(screen.getByText('disk full')).toBeTruthy()
  })
})

test('falls back to a generic message when the rejection carries nothing', async () => {
  vi.spyOn(console, 'error').mockImplementation(() => {})
  const sockets = useDrivableWebSocket()
  global.fetch = vi.fn().mockRejectedValue({})

  render(DocumentUpload, { props: { selectedCollection: 'test-collection' } })
  await attachFiles([pdf()])
  await fireEvent.click(screen.getByText('Upload 1 file'))

  sockets[0].onopen?.()

  await waitFor(() => {
    expect(screen.getByText('Failed to upload documents')).toBeTruthy()
  })
  expect(document.querySelector('.status')).toHaveClass('error')
})
