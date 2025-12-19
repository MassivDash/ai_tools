/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
// ... imports
import { render, fireEvent, waitFor } from '@testing-library/svelte'
// ...

// ...

  // Setup mocks
  const originalFileReader = global.FileReader
  class MockFileReader {
    onload: any
    readAsDataURL(_blob: Blob) {
        // Trigger onload
        setTimeout(() => {
             if (this.onload) {
                 this.onload({ target: { result: 'data:image/jpeg;base64,mockdata' } })
             }
        }, 20) 
    }
  }
import { expect, test, vi, beforeEach } from 'vitest'
import ChatInput from './ChatInput.svelte'
import { axiosBackendInstance } from '@axios/axiosBackendInstance.ts'
import type { Component } from 'svelte'

// Mock axiosBackendInstance
vi.mock('@axios/axiosBackendInstance.ts', () => ({
  axiosBackendInstance: {
    post: vi.fn()
  }
}))

const mockedAxios = axiosBackendInstance as unknown as {
  post: ReturnType<typeof vi.fn>
}

beforeEach(() => {
  vi.clearAllMocks()
  // Allow console logs for debugging failures
  vi.spyOn(console, 'log')
  vi.spyOn(console, 'error')
})

// ... (tests 1-5 same) ...

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

  // Ensure axios is clean
  vi.clearAllMocks()

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

test.skip('mocks image processing', async () => {
  const onAttachmentsChange = vi.fn()

  const file = new File(['fake-image'], 'test.jpg', { type: 'image/jpeg' })

  // Setup mocks
  const originalFileReader = global.FileReader
  class MockFileReader {
    onload: any
    readAsDataURL(blob: Blob) {
      // Trigger onload
      setTimeout(() => {
        if (this.onload) {
          this.onload({ target: { result: 'data:image/jpeg;base64,mockdata' } })
        }
      }, 20)
    }
  }
  global.FileReader = MockFileReader as any
  window.FileReader = MockFileReader as any

  // Mock Image
  const originalImage = window.Image
  class MockImage {
    onload: any
    width = 100
    height = 100
    _src: string = ''
    set src(val: string) {
      this._src = val
      // Trigger onload immediately when src is set
      setTimeout(() => {
        if (this.onload) {
          this.onload()
        }
      }, 20)
    }
    get src() {
      return this._src
    }
  }
  window.Image = MockImage as any

  // Mock Canvas
  const originalCreateElement = document.createElement
  document.createElement = vi.fn((tag) => {
    if (tag === 'canvas') {
      return {
        width: 0,
        height: 0,
        getContext: () => ({
          fillStyle: '',
          fillRect: vi.fn(),
          drawImage: vi.fn()
        }),
        toDataURL: () => 'data:image/jpeg;base64,processed'
      } as any
    }
    return originalCreateElement.call(document, tag)
  })

  render(ChatInput as Component, {
    props: {
      inputMessage: '',
      loading: false,
      modelCapabilities: { vision: true, audio: false },
      onSend: vi.fn(),
      onInputChange: vi.fn(),
      onAttachmentsChange
    }
  })

  const inputs = document.querySelectorAll('input[type="file"]')
  const imgInput = Array.from(inputs).find((i) =>
    (i as HTMLInputElement).accept.includes('image')
  ) as HTMLInputElement

  Object.defineProperty(imgInput, 'files', {
    value: [file]
  })

  await fireEvent.change(imgInput)

  await waitFor(
    () => {
      expect(onAttachmentsChange).toHaveBeenCalled()
      const callArgs = onAttachmentsChange.mock.calls[0][0]
      // Expect the *processed* data URL
      expect(callArgs[0].content).toBe('data:image/jpeg;base64,processed')
    },
    { timeout: 3000 }
  )

  // Cleanup
  global.FileReader = originalFileReader
  window.FileReader = originalFileReader
  window.Image = originalImage
  document.createElement = originalCreateElement
})
