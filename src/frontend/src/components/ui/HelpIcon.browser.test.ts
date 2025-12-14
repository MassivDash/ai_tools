/**
 * @vitest-environment jsdom
 */

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { expect, test } from 'vitest'
import HelpIcon from './HelpIcon.svelte'

test('renders help icon', () => {
  render(HelpIcon)

  const helpIcon = screen.getByLabelText('Help')
  expect(helpIcon).toBeTruthy()
  expect(helpIcon.textContent).toBe('?')
})

test('does not show tooltip by default', () => {
  render(HelpIcon, {
    props: { text: 'Help text' }
  })

  expect(screen.queryByText('Help text')).not.toBeInTheDocument()
})

test('shows tooltip on mouse enter', async () => {
  render(HelpIcon, {
    props: { text: 'Help text' }
  })

  const wrapper = screen.getByLabelText('Help').closest('.help-icon-wrapper')
  fireEvent.mouseEnter(wrapper!)

  await waitFor(() => {
    expect(screen.getByText('Help text')).toBeTruthy()
  })
})

test('hides tooltip on mouse leave', async () => {
  render(HelpIcon, {
    props: { text: 'Help text' }
  })

  const wrapper = screen.getByLabelText('Help').closest('.help-icon-wrapper')
  fireEvent.mouseEnter(wrapper!)

  await waitFor(() => {
    expect(screen.getByText('Help text')).toBeTruthy()
  })

  fireEvent.mouseLeave(wrapper!)

  await waitFor(() => {
    expect(screen.queryByText('Help text')).not.toBeInTheDocument()
  })
})

test('does not show tooltip when text is empty', async () => {
  const { container } = render(HelpIcon, {
    props: { text: '' }
  })

  const wrapper = container.querySelector('.help-icon-wrapper')
  fireEvent.mouseEnter(wrapper!)

  // Wait a bit to ensure tooltip doesn't appear
  await new Promise((resolve) => setTimeout(resolve, 100))

  const tooltip = container.querySelector('.tooltip')
  expect(tooltip).not.toBeInTheDocument()
})

test('wrapper has correct role and tabindex', () => {
  const { container } = render(HelpIcon)

  const wrapper = container.querySelector('.help-icon-wrapper')
  expect(wrapper).toHaveAttribute('role', 'button')
  expect(wrapper).toHaveAttribute('tabindex', '0')
})

test('help icon has correct aria-label', () => {
  render(HelpIcon)

  const helpIcon = screen.getByLabelText('Help')
  expect(helpIcon).toBeTruthy()
})
