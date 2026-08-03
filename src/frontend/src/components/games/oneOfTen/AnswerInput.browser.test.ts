// @vitest-environment jsdom

/// <reference types="@testing-library/jest-dom" />
import { render, screen, fireEvent } from '@testing-library/svelte'
import { describe, it, expect, vi } from 'vitest'
import AnswerInput from './AnswerInput.svelte'

const typeAnswer = async (value: string) => {
  const input = screen.getByPlaceholderText('Type your answer...')
  await fireEvent.input(input, { target: { value } })
  return input as HTMLInputElement
}

describe('AnswerInput', () => {
  it('submits the typed answer and clears the input', async () => {
    const onSubmit = vi.fn()
    render(AnswerInput, { props: { onSubmit } })

    const input = await typeAnswer('Warsaw')
    await fireEvent.click(screen.getByText('Submit'))

    expect(onSubmit).toHaveBeenCalledWith('Warsaw')
    expect(input.value).toBe('')
  })

  it('ignores submits when the answer is empty or whitespace only', async () => {
    const onSubmit = vi.fn()
    render(AnswerInput, { props: { onSubmit } })

    await fireEvent.click(screen.getByText('Submit'))
    expect(onSubmit).not.toHaveBeenCalled()

    await typeAnswer('   ')
    await fireEvent.click(screen.getByText('Submit'))
    expect(onSubmit).not.toHaveBeenCalled()
  })

  it('submits on Enter but not on other keys', async () => {
    const onSubmit = vi.fn()
    render(AnswerInput, { props: { onSubmit } })

    const input = await typeAnswer('Berlin')
    await fireEvent.keyDown(input, { key: 'a' })
    expect(onSubmit).not.toHaveBeenCalled()

    await fireEvent.keyDown(input, { key: 'Enter' })
    expect(onSubmit).toHaveBeenCalledWith('Berlin')
  })

  it('locks the field after a submit so the answer cannot be sent twice', async () => {
    const onSubmit = vi.fn()
    render(AnswerInput, { props: { onSubmit } })

    const input = await typeAnswer('Paris')
    await fireEvent.keyDown(input, { key: 'Enter' })

    expect(onSubmit).toHaveBeenCalledTimes(1)
    expect(input).toBeDisabled()
    expect(screen.getByText('Submit')).toBeDisabled()

    await fireEvent.keyDown(input, { key: 'Enter' })
    expect(onSubmit).toHaveBeenCalledTimes(1)
  })

  it('is disabled and does not submit when the disabled prop is set', async () => {
    const onSubmit = vi.fn()
    const { rerender } = render(AnswerInput, {
      props: { onSubmit, disabled: true }
    })

    const input = screen.getByPlaceholderText(
      'Type your answer...'
    ) as HTMLInputElement
    expect(input).toBeDisabled()
    expect(screen.getByText('Submit')).toBeDisabled()

    await fireEvent.keyDown(input, { key: 'Enter' })
    expect(onSubmit).not.toHaveBeenCalled()

    // Re-enabling clears the submitted lock so the next question can be answered
    await rerender({ onSubmit, disabled: false })
    await fireEvent.input(input, { target: { value: 'Rome' } })
    await fireEvent.click(screen.getByText('Submit'))
    expect(onSubmit).toHaveBeenCalledWith('Rome')
  })
})
