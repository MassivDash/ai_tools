/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import Select from './Select.svelte'

const mockOptions = [
  { value: 'option1', label: 'Option 1' },
  { value: 'option2', label: 'Option 2' },
  { value: 'option3', label: 'Option 3' }
]

test('renders select without label', () => {
  render(Select, {
    props: { options: mockOptions }
  })

  const select = screen.getByRole('combobox')
  expect(select).toBeTruthy()
})

test('renders select with label', () => {
  render(Select, {
    props: { label: 'Choose Option', id: 'select-id', options: mockOptions }
  })

  expect(screen.getByLabelText('Choose Option')).toBeTruthy()
  const select = screen.getByLabelText('Choose Option')
  expect(select).toHaveAttribute('id', 'select-id')
})

test('renders all options', () => {
  render(Select, {
    props: { options: mockOptions }
  })

  expect(screen.getByText('Option 1')).toBeTruthy()
  expect(screen.getByText('Option 2')).toBeTruthy()
  expect(screen.getByText('Option 3')).toBeTruthy()
})

test('selects correct value', () => {
  render(Select, {
    props: { value: 'option2', options: mockOptions }
  })

  const select = screen.getByRole('combobox') as HTMLSelectElement
  expect(select.value).toBe('option2')
})

test('dispatches change event when value changes', async () => {
  const handleChange = vi.fn()
  render(Select, {
    props: { options: mockOptions },
    events: { change: handleChange }
  })

  const select = screen.getByRole('combobox')
  fireEvent.change(select, { target: { value: 'option2' } })

  await waitFor(() => {
    expect(handleChange).toHaveBeenCalledTimes(1)
    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        detail: { value: 'option2' }
      })
    )
  })
})

test('updates value when changed', async () => {
  render(Select, {
    props: { value: 'option1', options: mockOptions }
  })

  const select = screen.getByRole('combobox') as HTMLSelectElement
  fireEvent.change(select, { target: { value: 'option3' } })

  await waitFor(() => {
    expect(select.value).toBe('option3')
  })
})

test('disables select when disabled prop is true', () => {
  render(Select, {
    props: { disabled: true, options: mockOptions }
  })

  const select = screen.getByRole('combobox')
  expect(select).toBeDisabled()
})

test('marks select as required when required prop is true', () => {
  render(Select, {
    props: { required: true, options: mockOptions }
  })

  const select = screen.getByRole('combobox')
  expect(select).toBeRequired()
})

test('handles empty options array', () => {
  render(Select, {
    props: { options: [] }
  })

  const select = screen.getByRole('combobox')
  expect(select).toBeTruthy()
  expect(select.querySelectorAll('option')).toHaveLength(0)
})

test('select has correct classes', () => {
  const { container } = render(Select, {
    props: { options: mockOptions }
  })

  const select = container.querySelector('.select')
  expect(select).toBeTruthy()
})

test('wrapper has correct class', () => {
  const { container } = render(Select, {
    props: { options: mockOptions }
  })

  const wrapper = container.querySelector('.select-wrapper')
  expect(wrapper).toBeTruthy()
})

test('label has correct class', () => {
  render(Select, {
    props: { label: 'Test Label', options: mockOptions }
  })

  const label = screen.getByText('Test Label')
  expect(label).toHaveClass('select-label')
})

test('handles multiple value changes', async () => {
  const handleChange = vi.fn()
  render(Select, {
    props: { options: mockOptions },
    events: { change: handleChange }
  })

  const select = screen.getByRole('combobox')

  fireEvent.change(select, { target: { value: 'option1' } })
  fireEvent.change(select, { target: { value: 'option2' } })
  fireEvent.change(select, { target: { value: 'option3' } })

  await waitFor(() => {
    expect(handleChange).toHaveBeenCalledTimes(3)
  })
})

test('renders without id', () => {
  const { container } = render(Select, {
    props: { label: 'Test', options: mockOptions }
  })

  // When id is empty string, label's for attribute will be empty
  // So we can't use getByLabelText, but we can check the structure
  const label = screen.getByText('Test')
  const select = container.querySelector('select')

  expect(label).toBeTruthy()
  expect(select).toBeTruthy()
  // Label should have for attribute (even if empty)
  expect(label).toHaveAttribute('for')
})
