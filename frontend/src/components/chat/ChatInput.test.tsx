import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ChatInput } from './ChatInput'

describe('ChatInput', () => {
  const setup = (props: Partial<Parameters<typeof ChatInput>[0]> = {}) => {
    const onSend = vi.fn()
    const result = render(<ChatInput onSend={onSend} {...props} />)
    const textarea: HTMLTextAreaElement = screen.getByRole('textbox')
    return { onSend, textarea, ...result }
  }

  it('renders textarea', () => {
    const { textarea } = setup()
    expect(textarea).toBeInTheDocument()
    expect(textarea).toHaveClass('chat-input__textarea')
  })

  it('typing updates the textarea value', () => {
    const { textarea } = setup()
    fireEvent.change(textarea, { target: { value: 'hello' } })
    expect(textarea.value).toBe('hello')
  })

  it('Enter key calls onSend with trimmed value', () => {
    const { textarea, onSend } = setup()
    fireEvent.change(textarea, { target: { value: '  hello  ' } })
    fireEvent.keyDown(textarea, { key: 'Enter' })
    expect(onSend).toHaveBeenCalledWith('hello')
  })

  it('Enter key clears input after send', () => {
    const { textarea, onSend } = setup()
    fireEvent.change(textarea, { target: { value: 'hello' } })
    fireEvent.keyDown(textarea, { key: 'Enter' })
    expect(onSend).toHaveBeenCalled()
    expect(textarea.value).toBe('')
  })

  it('Shift+Enter does not call onSend', () => {
    const { textarea, onSend } = setup()
    fireEvent.change(textarea, { target: { value: 'hello' } })
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: true })
    expect(onSend).not.toHaveBeenCalled()
    expect(textarea.value).toBe('hello')
  })

  it('does not send empty messages', () => {
    const { textarea, onSend } = setup()
    fireEvent.change(textarea, { target: { value: '   ' } })
    fireEvent.keyDown(textarea, { key: 'Enter' })
    expect(onSend).not.toHaveBeenCalled()
  })

  it('disabled prop disables textarea', () => {
    const { textarea } = setup({ disabled: true })
    expect(textarea).toBeDisabled()
  })
})
