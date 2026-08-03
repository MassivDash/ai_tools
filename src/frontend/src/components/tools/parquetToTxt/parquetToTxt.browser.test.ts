/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach, afterEach } from 'vitest'
import { tick } from 'svelte'
import ParquetToTxt from './parquetToTxt.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    post: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  post: ReturnType<typeof vi.fn>
}

let mockClick: ReturnType<typeof vi.fn>
let mockCreateElement: ReturnType<typeof vi.fn>
const originalCreateElement = document.createElement.bind(document)

/** Installs an anchor factory so downloads can be inspected without navigating. */
const stubAnchors = () => {
  mockClick = vi.fn()
  mockCreateElement = vi.fn((tagName: string) => {
    const el = originalCreateElement(tagName)
    if (tagName === 'a') el.click = mockClick
    return el
  })
  document.createElement =
    mockCreateElement as unknown as Document['createElement']
}

const parquetFile = (name = 'data.parquet', bytes = 1024) =>
  new File(['x'.repeat(bytes)], name, { type: 'application/parquet' })

const getFileInput = () =>
  screen.getByLabelText(/parquet/i, { selector: 'input' }) as HTMLInputElement

beforeEach(() => {
  vi.clearAllMocks()
  global.URL.createObjectURL = vi.fn(() => 'blob:mock-url')
  global.URL.revokeObjectURL = vi.fn()
  stubAnchors()
})

afterEach(() => {
  document.createElement = originalCreateElement as Document['createElement']
  vi.useRealTimers()
})

test('renders initial state with no file selected', () => {
  render(ParquetToTxt)

  expect(screen.getByText('Parquet to TXT Converter')).toBeTruthy()
  expect(screen.getByText('Choose parquet file(s)...')).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Convert' })).toBeDisabled()
  expect(screen.queryByText('Selected Files:')).not.toBeInTheDocument()
})

test('shows selected file count, formatted total size and per-file sizes', async () => {
  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), {
    target: {
      files: [parquetFile('a.parquet', 1024), parquetFile('b.parquet', 1024)]
    }
  })

  await waitFor(() => {
    expect(screen.getByText('2 file(s) selected (2 KB)')).toBeTruthy()
  })
  expect(screen.getByText('Selected Files:')).toBeTruthy()
  expect(screen.getByText('a.parquet (1 KB)')).toBeTruthy()
  expect(screen.getByText('b.parquet (1 KB)')).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Convert' })).not.toBeDisabled()
})

test('formats a zero-byte file as 0 Bytes', async () => {
  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), {
    target: { files: [new File([], 'empty.parquet')] }
  })

  await waitFor(() => {
    expect(screen.getByText('1 file(s) selected (0 Bytes)')).toBeTruthy()
  })
  expect(screen.getByText('empty.parquet (0 Bytes)')).toBeTruthy()
})

test('formats megabyte-sized selections with two decimals', async () => {
  render(ParquetToTxt)

  // 1.5 MiB -> "1.5 MB"
  await fireEvent.change(getFileInput(), {
    target: { files: [parquetFile('big.parquet', 1024 * 1024 * 1.5)] }
  })

  await waitFor(() => {
    expect(screen.getByText('1 file(s) selected (1.5 MB)')).toBeTruthy()
  })
})

test('ignores a change event that carries no files', async () => {
  render(ParquetToTxt)
  const input = getFileInput()

  await fireEvent.change(input, { target: { files: [parquetFile()] } })
  await waitFor(() => {
    expect(screen.getByText('1 file(s) selected (1 KB)')).toBeTruthy()
  })

  await fireEvent.change(input, { target: { files: [] } })

  // Selection is retained: the empty FileList is a no-op.
  expect(screen.getByText('1 file(s) selected (1 KB)')).toBeTruthy()
})

test('rejects non-parquet files without calling the API', async () => {
  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), {
    target: { files: [parquetFile('ok.parquet'), parquetFile('notes.txt')] }
  })

  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => {
    expect(
      screen.getByText(
        'Invalid file type. Please select only .parquet files. Found: notes.txt'
      )
    ).toBeTruthy()
  })
  expect(mockedAxios.post).not.toHaveBeenCalled()
})

test('clears a previous error when a new file is selected', async () => {
  render(ParquetToTxt)
  const input = getFileInput()

  await fireEvent.change(input, { target: { files: [parquetFile('bad.csv')] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))
  await waitFor(() => {
    expect(screen.getByText(/Invalid file type/)).toBeTruthy()
  })

  await fireEvent.change(input, { target: { files: [parquetFile()] } })

  await waitFor(() => {
    expect(screen.queryByText(/Invalid file type/)).not.toBeInTheDocument()
  })
})

test('posts every selected file as multipart form data expecting a blob', async () => {
  mockedAxios.post.mockResolvedValue({ data: 'txt-payload', headers: {} })

  render(ParquetToTxt)
  const fileA = parquetFile('a.parquet')
  const fileB = parquetFile('b.parquet')

  await fireEvent.change(getFileInput(), { target: { files: [fileA, fileB] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => {
    expect(mockedAxios.post).toHaveBeenCalledTimes(1)
  })

  const [url, body, config] = mockedAxios.post.mock.calls[0]
  expect(url).toBe('parquet-to-txt')
  expect(body).toBeInstanceOf(FormData)
  expect((body as FormData).getAll('files')).toEqual([fileA, fileB])
  expect(config.responseType).toBe('blob')
  expect(config.headers['Content-Type']).toBe('multipart/form-data')
})

test('shows converting state and disables the input while in flight', async () => {
  mockedAxios.post.mockImplementation(() => new Promise(() => {}))

  render(ParquetToTxt)
  const input = getFileInput()

  await fireEvent.change(input, { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Converting...' })).toBeDisabled()
  })
  expect(input).toBeDisabled()
  expect(screen.getByText('Preparing files...')).toBeTruthy()
})

test('renders percentage progress from onDownloadProgress when total is known', async () => {
  let capturedConfig: any
  mockedAxios.post.mockImplementation((_url, _body, config) => {
    capturedConfig = config
    return new Promise(() => {})
  })

  const { container } = render(ParquetToTxt)

  await fireEvent.change(getFileInput(), { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => expect(capturedConfig).toBeDefined())

  capturedConfig.onDownloadProgress({ loaded: 512, total: 2048 })
  await tick()

  await waitFor(() => {
    expect(screen.getByText('Downloading... 25%')).toBeTruthy()
  })
  expect(screen.getByText('25%')).toBeTruthy()
  expect(container.querySelector('.progress-bar')).toHaveAttribute(
    'style',
    'width: 25%;'
  )
})

test('falls back to megabytes downloaded when total is unknown', async () => {
  let capturedConfig: any
  mockedAxios.post.mockImplementation((_url, _body, config) => {
    capturedConfig = config
    return new Promise(() => {})
  })

  const { container } = render(ParquetToTxt)

  await fireEvent.change(getFileInput(), { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => expect(capturedConfig).toBeDefined())

  capturedConfig.onDownloadProgress({ loaded: 1024 * 1024 * 3, total: 0 })
  await tick()

  await waitFor(() => {
    expect(screen.getByText('Downloading... 3.00 MB')).toBeTruthy()
  })
  // No percentage is known, so no progress bar is drawn.
  expect(container.querySelector('.progress-bar')).toBeNull()
})

test('downloads the returned text using the filename from content-disposition', async () => {
  mockedAxios.post.mockResolvedValue({
    data: 'combined text',
    headers: { 'content-disposition': 'attachment; filename="combined.txt"' }
  })

  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => {
    expect(screen.getByText('✅ Download complete!')).toBeTruthy()
  })

  expect(mockCreateElement).toHaveBeenCalledWith('a')
  const anchor = mockCreateElement.mock.results
    .map((r) => r.value as HTMLElement)
    .find((el) => el.tagName === 'A') as HTMLAnchorElement
  // NOTE: the component's /filename="?(.+)"?/i is greedy, so a quoted header
  // value keeps its closing quote. Asserted as-is to pin current behaviour.
  expect(anchor.download).toBe('combined.txt"')
  expect(anchor.getAttribute('href')).toBe('blob:mock-url')
  expect(mockClick).toHaveBeenCalledTimes(1)
  expect(global.URL.createObjectURL).toHaveBeenCalledWith(expect.any(Blob))
  expect(global.URL.revokeObjectURL).toHaveBeenCalledWith('blob:mock-url')
  // The temporary anchor is cleaned up again.
  expect(document.body.contains(anchor)).toBe(false)
  expect(screen.getByText('100%')).toBeTruthy()
})

test('uses an unquoted content-disposition filename verbatim', async () => {
  mockedAxios.post.mockResolvedValue({
    data: 'combined text',
    headers: { 'content-disposition': 'attachment; filename=combined.txt' }
  })

  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => {
    expect(screen.getByText('✅ Download complete!')).toBeTruthy()
  })

  const anchor = mockCreateElement.mock.results
    .map((r) => r.value as HTMLElement)
    .find((el) => el.tagName === 'A') as HTMLAnchorElement
  expect(anchor.download).toBe('combined.txt')
})

test('uses the generated default filename when no filename header is present', async () => {
  mockedAxios.post.mockResolvedValue({
    data: 'combined text',
    headers: { 'content-disposition': 'attachment' }
  })

  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => {
    expect(screen.getByText('✅ Download complete!')).toBeTruthy()
  })

  const anchor = mockCreateElement.mock.results
    .map((r) => r.value as HTMLElement)
    .find((el) => el.tagName === 'A') as HTMLAnchorElement
  expect(anchor.download).toMatch(/^imatrix_quantization_data_\d+\.txt$/)
})

test('clears progress and re-enables converting two seconds after completion', async () => {
  vi.useFakeTimers()
  mockedAxios.post.mockResolvedValue({ data: 'txt', headers: {} })

  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await vi.advanceTimersByTimeAsync(0)
  expect(screen.getByText('✅ Download complete!')).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Converting...' })).toBeTruthy()

  await vi.advanceTimersByTimeAsync(2000)
  await tick()

  expect(screen.queryByText('✅ Download complete!')).not.toBeInTheDocument()
  expect(screen.queryByText('100%')).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Convert' })).not.toBeDisabled()
})

test('surfaces the API error payload and leaves the tool usable', async () => {
  mockedAxios.post.mockRejectedValue({
    response: { data: { error: 'Parquet schema mismatch' } },
    message: 'Request failed'
  })

  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => {
    expect(screen.getByText('Parquet schema mismatch')).toBeTruthy()
  })
  expect(screen.getByRole('button', { name: 'Convert' })).not.toBeDisabled()
  expect(mockClick).not.toHaveBeenCalled()
})

test('falls back to the error message when no response payload exists', async () => {
  mockedAxios.post.mockRejectedValue({ message: 'Network Error' })

  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => {
    expect(screen.getByText('Network Error')).toBeTruthy()
  })
})

test('falls back to a generic message for an opaque failure', async () => {
  mockedAxios.post.mockRejectedValue({})

  render(ParquetToTxt)

  await fireEvent.change(getFileInput(), { target: { files: [parquetFile()] } })
  await fireEvent.click(screen.getByRole('button', { name: 'Convert' }))

  await waitFor(() => {
    expect(
      screen.getByText('Failed to convert parquet files to text')
    ).toBeTruthy()
  })
})
