import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, fireEvent, act } from '@testing-library/react'
import { RichChatInput } from './RichChatInput'
import { contextMentionStore } from '@/stores'
import type { PickableEntity } from '@/stores/contextMentionStore'

const makeEntity = (id: string, name: string): PickableEntity => ({
  kind: 'context-node',
  id,
  name,
  summary: `Context: ${name}`,
  data: { content: `content of ${name}` },
})

const flushMicrotasks = async () => {
  await act(async () => {
    await new Promise((r) => {
      setTimeout(r, 0)
    })
  })
}

describe('RichChatInput', () => {
  beforeEach(() => {
    contextMentionStore.reset()
  })

  it('renders with placeholder when empty', () => {
    render(<RichChatInput onSend={vi.fn()} stepId="step1" />)
    expect(screen.getByText('Type a message...')).toBeInTheDocument()
  })

  it('renders with custom placeholder', () => {
    render(<RichChatInput onSend={vi.fn()} stepId="step1" placeholder="Custom..." />)
    expect(screen.getByText('Custom...')).toBeInTheDocument()
  })

  it('renders a contentEditable div', () => {
    render(<RichChatInput onSend={vi.fn()} stepId="step1" />)
    const input = screen.getByRole('textbox')
    expect(input).toBeInTheDocument()
    expect(input.getAttribute('contenteditable')).toBe('true')
  })

  it('renders contentEditable as false when disabled', () => {
    render(<RichChatInput onSend={vi.fn()} stepId="step1" disabled />)
    const input = screen.getByRole('textbox')
    expect(input.getAttribute('contenteditable')).toBe('false')
  })

  it('calls onSend with text on Enter', () => {
    const onSend = vi.fn()
    render(<RichChatInput onSend={onSend} stepId="step1" />)
    const input = screen.getByRole('textbox')

    act(() => {
      input.textContent = 'Hello world'
      fireEvent.input(input)
    })

    act(() => {
      fireEvent.keyDown(input, { key: 'Enter' })
    })

    expect(onSend).toHaveBeenCalledWith('Hello world')
  })

  it('does not send on Shift+Enter', () => {
    const onSend = vi.fn()
    render(<RichChatInput onSend={onSend} stepId="step1" />)
    const input = screen.getByRole('textbox')

    act(() => {
      input.textContent = 'Hello'
      fireEvent.input(input)
    })

    act(() => {
      fireEvent.keyDown(input, { key: 'Enter', shiftKey: true })
    })

    expect(onSend).not.toHaveBeenCalled()
  })

  it('does not send empty content', () => {
    const onSend = vi.fn()
    render(<RichChatInput onSend={onSend} stepId="step1" />)
    const input = screen.getByRole('textbox')

    act(() => {
      fireEvent.keyDown(input, { key: 'Enter' })
    })

    expect(onSend).not.toHaveBeenCalled()
  })

  it('clears input after sending', () => {
    const onSend = vi.fn()
    render(<RichChatInput onSend={onSend} stepId="step1" />)
    const input = screen.getByRole('textbox')

    act(() => {
      input.textContent = 'test'
      fireEvent.input(input)
    })

    act(() => {
      fireEvent.keyDown(input, { key: 'Enter' })
    })

    expect(input.innerHTML).toBe('')
  })

  it('inserts chip when mention is added to store', async () => {
    render(<RichChatInput onSend={vi.fn()} stepId="step1" />)
    const input = screen.getByRole('textbox')

    act(() => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'My Context'), '#10b981')
    })
    await flushMicrotasks()

    const chip = input.querySelector('[data-mention-id]')
    expect(chip).not.toBeNull()
    expect(chip?.textContent).toContain('My Context')
  })

  it('removes chip when mention is removed from store', async () => {
    render(<RichChatInput onSend={vi.fn()} stepId="step1" />)
    const input = screen.getByRole('textbox')

    act(() => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'My Context'), '#10b981')
    })
    await flushMicrotasks()

    const mentions = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
    const tokenId = mentions[0]!.id

    expect(input.querySelector('[data-mention-id]')).not.toBeNull()

    act(() => {
      contextMentionStore.removeMention('step1', tokenId)
    })
    await flushMicrotasks()

    expect(input.querySelector('[data-mention-id]')).toBeNull()
  })

  it('removes chip via X button click', async () => {
    render(<RichChatInput onSend={vi.fn()} stepId="step1" />)
    const input = screen.getByRole('textbox')

    act(() => {
      contextMentionStore.addMention('step1', makeEntity('e1', 'Test'), '#10b981')
    })
    await flushMicrotasks()

    const mentions = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
    const tokenId = mentions[0]!.id

    const removeBtn = input.querySelector(`[data-remove-mention="${tokenId}"]`) as HTMLElement
    expect(removeBtn).not.toBeNull()

    act(() => {
      fireEvent.click(removeBtn)
    })
    await flushMicrotasks()

    const remaining = contextMentionStore.selectMentions('step1')(contextMentionStore.store.getState())
    expect(remaining).toHaveLength(0)
  })

  it('does not send when disabled', () => {
    const onSend = vi.fn()
    render(<RichChatInput onSend={onSend} stepId="step1" disabled />)
    const input = screen.getByRole('textbox')

    act(() => {
      input.textContent = 'test'
      fireEvent.keyDown(input, { key: 'Enter' })
    })

    expect(onSend).not.toHaveBeenCalled()
  })
})
