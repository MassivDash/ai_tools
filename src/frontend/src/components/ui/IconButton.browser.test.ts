/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, fireEvent } from '@testing-library/svelte'
import { expect, test, vi } from 'vitest'
import IconButton from './IconButton.svelte'

test('renders icon button', () => {
  const { container } = render(IconButton, {
    props: { children: () => 'Icon' }
  })

  const button = container.querySelector('button')
  expect(button).toBeTruthy()
  expect(button).toHaveClass('button-icon-only')
})

test('renders with default variant', () => {
  const { container } = render(IconButton, {
    props: { children: () => 'Icon' }
  })

  const button = container.querySelector('button')
  expect(button).toHaveClass('button-info')
})

test('renders with different variants', () => {
  const { container } = render(IconButton, {
    props: { variant: 'primary', children: () => 'Icon' }
  })

  const button = container.querySelector('button')
  expect(button).toHaveClass('button-primary')
  expect(button).toHaveClass('button-icon-only')
})

test('disables button when disabled prop is true', () => {
  const { container } = render(IconButton, {
    props: { disabled: true, children: () => 'Icon' }
  })

  const button = container.querySelector('button')
  expect(button).toBeDisabled()
})

test('applies title attribute', () => {
  const { container } = render(IconButton, {
    props: { title: 'Tooltip text', children: () => 'Icon' }
  })

  const button = container.querySelector('button')
  expect(button).toHaveAttribute('title', 'Tooltip text')
})

test('handles click events', () => {
  const handleClick = vi.fn()
  const { container } = render(IconButton, {
    props: { children: () => 'Icon', onclick: handleClick }
  })

  const button = container.querySelector('button')
  fireEvent.click(button!)
  expect(handleClick).toHaveBeenCalledTimes(1)
})

test('applies all variant classes', () => {
  const variants: Array<
    'primary' | 'secondary' | 'success' | 'danger' | 'info'
  > = ['primary', 'secondary', 'success', 'danger', 'info']

  variants.forEach((variant) => {
    const { container, unmount } = render(IconButton, {
      props: { variant, children: () => 'Icon' }
    })

    const button = container.querySelector('button')
    expect(button).toHaveClass(`button-${variant}`)
    expect(button).toHaveClass('button-icon-only')
    unmount()
  })
})
