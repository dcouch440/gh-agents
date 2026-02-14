// ============================================================================
// contentExtraction — Walk contentEditable DOM to produce final message string
// ============================================================================

import type { MentionToken } from '@/stores/contextMentionStore'
import { formatEntityContext } from '@/utils/formatEntityContext'
import { ZERO_WIDTH_SPACE } from './chipInsertion'

const extractContent = (container: HTMLElement, mentions: ReadonlyArray<MentionToken>): string => {
  const parts: string[] = []

  const walk = (node: Node): void => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = (node.textContent ?? '').replaceAll(ZERO_WIDTH_SPACE, '')
      if (text) parts.push(text)
      return
    }

    if (!(node instanceof HTMLElement)) return

    const mentionId = node.getAttribute('data-mention-id')
    if (mentionId) {
      const token = mentions.find((m) => m.id === mentionId)
      if (token) {
        parts.push(formatEntityContext(token.entity))
      }
      return
    }

    if (node.tagName === 'BR') {
      parts.push('\n')
      return
    }

    if (node.tagName === 'DIV' || node.tagName === 'P') {
      if (parts.length > 0 && !parts[parts.length - 1]!.endsWith('\n')) {
        parts.push('\n')
      }
    }

    for (const child of node.childNodes) {
      walk(child)
    }
  }

  for (const child of container.childNodes) {
    walk(child)
  }

  return parts.join('')
}

export { extractContent }
