import { useCallback, useEffect, useRef, useState } from 'react'
import { Box } from '@mui/material'
import { useStore, contextMentionStore } from '@/stores'
import { extractContent } from './contentExtraction'
import { insertChipAtCursor, removeChipFromDOM } from './chipInsertion'

type RichChatInputProps = {
  onSend: (message: string) => void
  stepId: string
  disabled?: boolean
  placeholder?: string
  focusMode?: boolean
}

function RichChatInput({ onSend, stepId, disabled, placeholder = 'Type a message...', focusMode }: RichChatInputProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const insertedRef = useRef<Set<string>>(new Set())
  const [isEmpty, setIsEmpty] = useState(true)

  const mentions = useStore(contextMentionStore.store, contextMentionStore.selectMentions(stepId))

  const checkEmpty = useCallback(() => {
    const el = containerRef.current
    if (!el) return
    const text = (el.textContent || '').replaceAll('\u200B', '').trim()
    const hasChip = el.querySelector('[data-mention-id]') !== null
    setIsEmpty(!text && !hasChip)
  }, [])

  // Sync chips: insert new mentions, remove deleted ones
  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const currentIds = new Set(mentions.map((m) => m.id))

    // Insert newly added mentions
    for (const mention of mentions) {
      if (!insertedRef.current.has(mention.id)) {
        insertChipAtCursor(container, mention)
        insertedRef.current.add(mention.id)
      }
    }

    // Remove mentions deleted from store
    for (const id of insertedRef.current) {
      if (!currentIds.has(id)) {
        removeChipFromDOM(container, id)
        insertedRef.current.delete(id)
      }
    }

    // Defer isEmpty check to avoid setState-in-effect lint rule
    queueMicrotask(checkEmpty)
  }, [mentions, checkEmpty])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      if (disabled) return

      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault()
        const container = containerRef.current
        if (!container) return

        const content = extractContent(container, mentions)
        if (content.trim()) {
          onSend(content)
          container.innerHTML = ''
          insertedRef.current.clear()
          setIsEmpty(true)
        }
        return
      }

      if (e.key === 'Backspace') {
        const sel = window.getSelection()
        if (!sel || sel.rangeCount === 0) return
        const range = sel.getRangeAt(0)
        if (!range.collapsed) return

        const node = range.startContainer
        const offset = range.startOffset

        // Cursor at start of a text node — check if previous sibling is a chip
        if (node.nodeType === Node.TEXT_NODE && offset === 0) {
          const prev = node.previousSibling
          if (prev instanceof HTMLElement && prev.hasAttribute('data-mention-id')) {
            e.preventDefault()
            const mentionId = prev.getAttribute('data-mention-id')!
            contextMentionStore.removeMention(stepId, mentionId)
            return
          }
        }

        // Cursor in a zero-width space after a chip
        if (node.nodeType === Node.TEXT_NODE && node.textContent === '\u200B' && offset <= 1) {
          const prev = node.previousSibling
          if (prev instanceof HTMLElement && prev.hasAttribute('data-mention-id')) {
            e.preventDefault()
            const mentionId = prev.getAttribute('data-mention-id')!
            contextMentionStore.removeMention(stepId, mentionId)
            return
          }
        }
      }
    },
    [disabled, mentions, onSend, stepId],
  )

  const handleInput = useCallback(() => {
    checkEmpty()
  }, [checkEmpty])

  const handlePaste = useCallback((e: React.ClipboardEvent<HTMLDivElement>) => {
    e.preventDefault()
    const text = e.clipboardData.getData('text/plain')
    document.execCommand('insertText', false, text)
  }, [])

  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const target = e.target as HTMLElement
      const removeId = target.getAttribute('data-remove-mention')
      if (removeId) {
        e.preventDefault()
        e.stopPropagation()
        contextMentionStore.removeMention(stepId, removeId)
      }
    },
    [stepId],
  )

  return (
    <Box
      sx={{
        position: 'relative',
        px: focusMode ? 2 : 1.5,
        py: focusMode ? 1.5 : 1,
        cursor: 'text',
        ...(focusMode && {
          mx: 3,
          mb: 2,
          borderRadius: '8px',
          border: 1,
          borderColor: 'divider',
          backgroundColor: (t) => t.palette.custom.bgPanel,
        }),
      }}
      onClick={() => {
        containerRef.current?.focus()
      }}
    >
      {/* Placeholder */}
      {isEmpty && (
        <Box
          component="span"
          sx={{
            position: 'absolute',
            top: '50%',
            transform: 'translateY(-50%)',
            pointerEvents: 'none',
            fontFamily: 'monospace',
            fontSize: '0.8125rem',
            color: 'text.secondary',
            opacity: 0.4,
          }}
        >
          {placeholder}
        </Box>
      )}

      {/* Editable area */}
      <Box
        ref={containerRef}
        component="div"
        contentEditable={!disabled}
        suppressContentEditableWarning
        role="textbox"
        aria-multiline="true"
        aria-label="Chat message input"
        onKeyDown={handleKeyDown}
        onInput={handleInput}
        onPaste={handlePaste}
        onClick={handleClick}
        sx={{
          minHeight: 24,
          maxHeight: 96,
          overflowY: 'auto',
          fontFamily: 'monospace',
          fontSize: '0.8125rem',
          lineHeight: 1.5,
          outline: 'none',
          py: 0.75,
          wordBreak: 'break-word',
          whiteSpace: 'pre-wrap',
          color: 'text.primary',
          '&:empty': {
            minHeight: 24,
          },
        }}
      />
    </Box>
  )
}

export { RichChatInput }
export type { RichChatInputProps }
