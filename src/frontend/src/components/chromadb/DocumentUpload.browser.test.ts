/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi, beforeEach } from 'vitest'
import DocumentUpload from './DocumentUpload.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    post: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as {
  post: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  global.URL.createObjectURL = vi.fn(() => 'blob:mock-url')
  global.URL.revokeObjectURL = vi.fn()
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
  mockedAxios.post.mockResolvedValueOnce({
    data: {
      success: true,
      message: 'Upload successful'
    }
  })

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
    expect(mockedAxios.post).toHaveBeenCalledWith(
      'chromadb/documents/upload',
      expect.any(FormData),
      expect.objectContaining({
        headers: {
          'Content-Type': 'multipart/form-data'
        }
      })
    )
  })

  await waitFor(() => {
    expect(handleUploaded).toHaveBeenCalled()
  })
})

test('shows error when upload fails', async () => {
  mockedAxios.post.mockResolvedValueOnce({
    data: {
      success: false,
      error: 'Upload failed'
    }
  })

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
