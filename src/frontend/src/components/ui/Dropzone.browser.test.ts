/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import Dropzone from './Dropzone.svelte'

test('renders dropzone with default props', () => {
  render(Dropzone)
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  expect(dropzone).toBeTruthy()
  expect(dropzone).toHaveClass('dropzone')
  
  expect(screen.getByText('Drag and drop files here, or')).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Browse Files' })).toBeTruthy()
})

test('renders with custom button text', () => {
  render(Dropzone, {
    props: { buttonText: 'Select Files' }
  })
  
  expect(screen.getByRole('button', { name: 'Select Files' })).toBeTruthy()
})

test('renders hint when provided', () => {
  render(Dropzone, {
    props: { hint: 'Only PDF files are allowed' }
  })
  
  expect(screen.getByText('Only PDF files are allowed')).toBeTruthy()
})

test('does not render hint when not provided', () => {
  render(Dropzone)
  
  expect(screen.queryByText(/hint/i)).not.toBeInTheDocument()
})

test('file input has correct attributes', () => {
  render(Dropzone, {
    props: { accept: '.pdf,.txt', multiple: false }
  })
  
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  expect(fileInput).toBeTruthy()
  expect(fileInput).toHaveAttribute('accept', '.pdf,.txt')
  expect(fileInput).not.toHaveAttribute('multiple')
})

test('file input supports multiple files by default', () => {
  render(Dropzone)
  
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  expect(fileInput).toHaveAttribute('multiple')
})

test('dispatches files event when files are selected via input', async () => {
  const handleFiles = vi.fn()
  render(Dropzone, {
    props: {},
    events: { files: handleFiles }
  })
  
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  const file = new File(['content'], 'test.txt', { type: 'text/plain' })
  const fileList = Object.assign([file], {
    item: (index: number) => fileList[index] || null,
    length: 1
  })
  
  Object.defineProperty(fileInput, 'files', {
    value: fileList,
    writable: false
  })
  
  fireEvent.change(fileInput)
  
  await waitFor(() => {
    expect(handleFiles).toHaveBeenCalledTimes(1)
    expect(handleFiles).toHaveBeenCalledWith(
      expect.objectContaining({
        detail: expect.arrayContaining([
          expect.objectContaining({
            name: 'test.txt'
          })
        ])
      })
    )
  })
})

test('dispatches files event when files are dropped', async () => {
  const handleFiles = vi.fn()
  render(Dropzone, {
    props: {},
    events: { files: handleFiles }
  })
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  const file = new File(['content'], 'dropped.txt', { type: 'text/plain' })
  
  const dataTransfer = {
    files: [file],
    items: [],
    types: ['Files']
  }
  
  const dropEvent = new Event('drop', { bubbles: true, cancelable: true }) as DragEvent
  Object.defineProperty(dropEvent, 'dataTransfer', {
    value: dataTransfer,
    writable: false
  })
  
  fireEvent(dropzone, dropEvent)
  
  await waitFor(() => {
    expect(handleFiles).toHaveBeenCalledTimes(1)
    expect(handleFiles).toHaveBeenCalledWith(
      expect.objectContaining({
        detail: expect.arrayContaining([
          expect.objectContaining({
            name: 'dropped.txt'
          })
        ])
      })
    )
  })
})

test('activates dropzone on dragenter', () => {
  render(Dropzone)
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  expect(dropzone).not.toHaveClass('active')
  
  const dragEnterEvent = new Event('dragenter', { bubbles: true, cancelable: true }) as DragEvent
  fireEvent(dropzone, dragEnterEvent)
  
  expect(dropzone).toHaveClass('active')
})

test('deactivates dropzone on dragleave', () => {
  render(Dropzone)
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  
  // First activate it
  fireEvent(dropzone, new Event('dragenter', { bubbles: true, cancelable: true }))
  expect(dropzone).toHaveClass('active')
  
  // Then deactivate it
  fireEvent(dropzone, new Event('dragleave', { bubbles: true, cancelable: true }))
  expect(dropzone).not.toHaveClass('active')
})

test('triggers file input when dropzone is clicked', () => {
  render(Dropzone)
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  const clickSpy = vi.spyOn(fileInput, 'click')
  
  // Click on the dropzone (not the button)
  fireEvent.click(dropzone)
  
  expect(clickSpy).toHaveBeenCalledTimes(1)
})

test('triggers file input when browse button is clicked', () => {
  render(Dropzone)
  
  const browseButton = screen.getByRole('button', { name: 'Browse Files' })
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  const clickSpy = vi.spyOn(fileInput, 'click')
  
  fireEvent.click(browseButton)
  
  expect(clickSpy).toHaveBeenCalledTimes(1)
})

test('triggers file input on Enter key', () => {
  render(Dropzone)
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  const clickSpy = vi.spyOn(fileInput, 'click')
  
  dropzone.focus()
  fireEvent.keyDown(dropzone, { key: 'Enter' })
  
  expect(clickSpy).toHaveBeenCalledTimes(1)
})

test('triggers file input on Space key', () => {
  render(Dropzone)
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  const clickSpy = vi.spyOn(fileInput, 'click')
  
  dropzone.focus()
  fireEvent.keyDown(dropzone, { key: ' ' })
  
  expect(clickSpy).toHaveBeenCalledTimes(1)
})

test('does not trigger file input on other keys', () => {
  render(Dropzone)
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  const clickSpy = vi.spyOn(fileInput, 'click')
  
  dropzone.focus()
  fireEvent.keyDown(dropzone, { key: 'a' })
  
  expect(clickSpy).not.toHaveBeenCalled()
})

test('does not trigger actions when disabled', () => {
  render(Dropzone, {
    props: { disabled: true }
  })
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  const clickSpy = vi.spyOn(fileInput, 'click')
  
  expect(dropzone).toHaveClass('disabled')
  expect(dropzone).toHaveAttribute('tabindex', '-1')
  
  // Try clicking
  fireEvent.click(dropzone)
  expect(clickSpy).not.toHaveBeenCalled()
  
  // Try keyboard
  dropzone.focus()
  fireEvent.keyDown(dropzone, { key: 'Enter' })
  expect(clickSpy).not.toHaveBeenCalled()
  
  // Try drag
  fireEvent(dropzone, new Event('dragenter', { bubbles: true, cancelable: true }))
  expect(dropzone).not.toHaveClass('active')
})

test('does not trigger dropzone click when button is clicked', () => {
  render(Dropzone)
  
  const dropzone = screen.getByRole('button', { name: 'Drop files here or click to browse' })
  const browseButton = screen.getByRole('button', { name: 'Browse Files' })
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  const clickSpy = vi.spyOn(fileInput, 'click')
  
  // Click the browse button - should only trigger once (from button handler, not dropzone)
  fireEvent.click(browseButton)
  
  expect(clickSpy).toHaveBeenCalledTimes(1)
})

test('does not dispatch files event when no files are selected', async () => {
  const handleFiles = vi.fn()
  render(Dropzone, {
    props: {},
    events: { files: handleFiles }
  })
  
  const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement
  
  // Create empty file list
  Object.defineProperty(fileInput, 'files', {
    value: [],
    writable: false
  })
  
  fireEvent.change(fileInput)
  
  // Wait a bit to ensure no event is dispatched
  await new Promise(resolve => setTimeout(resolve, 100))
  
  expect(handleFiles).not.toHaveBeenCalled()
})

